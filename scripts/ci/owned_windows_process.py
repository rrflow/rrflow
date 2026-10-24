"""Bound a qualification subprocess tree before any child code executes.

Uses the existing Windows qualification ABI definitions; it creates no principal,
ACL, firewall rule, service or product dependency. Output stays in bounded pipes.
"""
import ctypes as c
from ctypes import wintypes as w
from contextlib import ExitStack
import os
import subprocess
import threading
import time

from run_offline_lifecycle import require
from windows_offline import (Information, JobLimits, P, Startup, U32, api, boolean,
                             close, kernel, quiesce_job)


def run_owned(arguments, *, env, timeout=600, limit=8 * 1024 * 1024,
              process_memory_bytes=None, job_memory_bytes=None, on_bound=None, job_name=None, cpu_rate=None):
    import msvcrt
    streams = [bytearray(), bytearray()]
    exceeded = threading.Event()
    readers = []
    if cpu_rate is not None:
        require(isinstance(cpu_rate, int) and 0 < cpu_rate <= 10000, "invalid native job CPU quota")
    if process_memory_bytes is not None or job_memory_bytes is not None:
        require(isinstance(process_memory_bytes, int) and isinstance(job_memory_bytes, int)
                and 0 < process_memory_bytes <= job_memory_bytes < 2**63,
                "invalid native job memory allocation")
    if job_name is not None:
        suffix=job_name.removeprefix("Local\\RRFlowCapacity-")
        require(job_name.startswith("Local\\RRFlowCapacity-") and len(suffix)==64
                and all(c in "0123456789abcdef" for c in suffix), "invalid owned job identity")
    with ExitStack() as stack:
        writers = stack.enter_context(ExitStack())
        c.set_last_error(0)
        job = api(kernel, "CreateJobObjectW", P, P, w.LPCWSTR)(None, job_name)
        existed=c.get_last_error()==183
        boolean(job, "create capacity process job")
        if existed:
            close(job)
            raise RuntimeError("refuse an existing named job")
        # Child handles and output pipes must stay valid until the tree quiesces.
        limits = JobLimits()
        limits.basic.flags = 0x2000
        if process_memory_bytes is not None or job_memory_bytes is not None:
            # Windows limits committed virtual memory, not an RSS estimate.
            limits.basic.flags |= 0x100 | 0x200
            limits.process_memory = process_memory_bytes
            limits.job_memory = job_memory_bytes
        try:
            boolean(api(kernel, "SetInformationJobObject", w.BOOL, P, U32, P, U32)(
                job, 9, c.byref(limits), c.sizeof(limits)), "bind capacity job lifetime")
            if cpu_rate is not None:
                # JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | HARD_CAP, hundredths of percent.
                cpu = (U32 * 2)(0x1 | 0x4, cpu_rate)
                boolean(api(kernel, "SetInformationJobObject", w.BOOL, P, U32, P, U32)(
                    job, 15, cpu, c.sizeof(cpu)), "bind capacity job CPU quota")
            startup, information = Startup(), Information()
            startup.size, startup.flags, startup.show = c.sizeof(startup), 0x101, 0
            null = stack.enter_context(open(os.devnull, "rb"))
            startup.stdin = msvcrt.get_osfhandle(null.fileno())
            os.set_handle_inheritable(startup.stdin, True)
            for index, field in enumerate(["stdout", "stderr"]):
                read_fd, write_fd = os.pipe()
                source = stack.enter_context(os.fdopen(read_fd, "rb", buffering=0))
                writers.callback(os.close, write_fd)
                handle = msvcrt.get_osfhandle(write_fd)
                os.set_handle_inheritable(handle, True)
                setattr(startup, field, handle)

                def capture(pipe=source, destination=streams[index]):
                    try:
                        while chunk := pipe.read(65536):
                            if len(destination) + len(chunk) > limit:
                                exceeded.set()
                                return
                            destination.extend(chunk)
                    except OSError:
                        exceeded.set()

                reader = threading.Thread(target=capture, daemon=True)
                readers.append(reader)
                reader.start()
            command = c.create_unicode_buffer(subprocess.list2cmdline([str(a) for a in arguments]))
            block = c.create_unicode_buffer("\0".join(f"{k}={v}" for k, v in sorted(env.items())) + "\0\0")
            try:
                boolean(api(kernel, "CreateProcessW", w.BOOL, w.LPCWSTR, w.LPWSTR,
                            P, P, w.BOOL, U32, P, w.LPCWSTR, P, P)(
                    str(arguments[0]), command, None, None, True, 0x08000404, block, None,
                    c.byref(startup), c.byref(information)), "create suspended capacity child")
            finally:
                writers.close()
                os.set_handle_inheritable(startup.stdin, False)
            stack.callback(close, information.process)
            stack.callback(close, information.thread)
            # Assignment failure must also terminate the suspended child.
            stack.callback(api(kernel, "TerminateProcess", w.BOOL, P, U32), information.process, 1)
            boolean(api(kernel, "AssignProcessToJobObject", w.BOOL, P, P)(job, information.process),
                    "assign capacity child before execution")
            if on_bound is not None:
                on_bound()
            require(api(kernel, "ResumeThread", U32, P)(information.thread) != 0xFFFFFFFF,
                    "resume capacity child")
            wait = api(kernel, "WaitForSingleObject", U32, P, U32)
            deadline = time.monotonic() + timeout
            while (state := wait(information.process, 50)) == 258:
                require(not exceeded.is_set(), "capacity child output exceeded bound")
                require(time.monotonic() < deadline, "capacity child exceeded deadline")
            require(state == 0, "capacity child wait failed")
            code = U32()
            boolean(api(kernel, "GetExitCodeProcess", w.BOOL, P, P)(information.process, c.byref(code)),
                    "read capacity child exit")
        finally:
            writers.close()
            quiesce_job(job)
            for reader in readers:
                reader.join(timeout=2)
        require(not exceeded.is_set() and not any(r.is_alive() for r in readers),
                "capacity output incomplete or exceeded bound")
    return subprocess.CompletedProcess(arguments, code.value,
                                       streams[0].decode("utf-8", errors="replace"),
                                       streams[1].decode("utf-8", errors="replace"))
