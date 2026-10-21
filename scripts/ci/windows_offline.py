"""CI-only Windows qualification principal, dynamic WFP filters and owned job.

No product dependency. No machine-wide firewall rule. The temporary principal's
token cannot bypass directory traversal, so a deny on each source root also
denies direct descendant access. Every resource below belongs to this invocation.
"""
import ctypes as c
from ctypes import wintypes as w
from contextlib import ExitStack
import os
import encodings
from pathlib import Path
import secrets
import subprocess
import sys
import time
import uuid

from run_offline_lifecycle import require


P = c.c_void_p
U32 = c.c_uint32
U64 = c.c_uint64
SIZE = c.c_size_t
adv = c.WinDLL("advapi32", use_last_error=True)
kernel = c.WinDLL("kernel32", use_last_error=True)
net = c.WinDLL("netapi32", use_last_error=True)
wfp = c.WinDLL("fwpuclnt", use_last_error=True)
user32 = c.WinDLL("user32", use_last_error=True)


def api(library, name, result, *arguments):
    function = getattr(library, name)
    function.restype = result
    function.argtypes = arguments
    return function


def checked(status, operation):
    require(status == 0, f"{operation} failed with Windows status {status}")


def boolean(status, operation):
    require(bool(status), f"{operation} failed with Windows error {c.get_last_error()}")


class Guid(c.Structure):
    _fields_ = [("bytes", c.c_ubyte * 16)]

    @classmethod
    def value(cls, text):
        return cls.from_buffer_copy(uuid.UUID(text).bytes_le)


class Display(c.Structure):
    _fields_ = [("name", w.LPWSTR), ("description", w.LPWSTR)]


class Blob(c.Structure):
    _fields_ = [("size", U32), ("data", P)]


class ValueData(c.Union):
    _fields_ = [("pointer", P), ("number", U32), ("alignment", U64)]


class Value(c.Structure):
    _fields_ = [("type", U32), ("value", ValueData)]


class Condition(c.Structure):
    _fields_ = [("field", Guid), ("match", U32), ("value", Value)]


class Action(c.Structure):
    _fields_ = [("type", U32), ("key", Guid)]


class Context(c.Union):
    _fields_ = [("raw", U64), ("key", Guid)]


class Filter(c.Structure):
    _fields_ = [("key", Guid), ("display", Display), ("flags", U32), ("provider", P),
                ("data", Blob), ("layer", Guid), ("sublayer", Guid), ("weight", Value),
                ("count", U32), ("conditions", c.POINTER(Condition)), ("action", Action),
                ("context", Context), ("reserved", P), ("id", U64), ("effective_weight", Value)]


class Session(c.Structure):
    _fields_ = [("key", Guid), ("display", Display), ("flags", U32), ("timeout", U32),
                ("pid", U32), ("sid", P), ("username", w.LPWSTR), ("kernel", w.BOOL)]


class User(c.Structure):
    _fields_ = [("name", w.LPWSTR), ("password", w.LPWSTR), ("age", U32), ("privilege", U32),
                ("home", w.LPWSTR), ("comment", w.LPWSTR), ("flags", U32), ("script", w.LPWSTR)]


class Trustee(c.Structure):
    _fields_ = [("multiple", P), ("operation", U32), ("form", U32), ("type", U32), ("name", P)]


class Access(c.Structure):
    _fields_ = [("permissions", U32), ("mode", U32), ("inheritance", U32), ("trustee", Trustee)]


class Luid(c.Structure):
    _fields_ = [("low", U32), ("high", c.c_int32)]


class Privilege(c.Structure):
    _fields_ = [("luid", Luid), ("attributes", U32)]


class Startup(c.Structure):
    _fields_ = [("size", U32), ("reserved", w.LPWSTR), ("desktop", w.LPWSTR), ("title", w.LPWSTR),
                ("x", U32), ("y", U32), ("width", U32), ("height", U32), ("chars_x", U32),
                ("chars_y", U32), ("fill", U32), ("flags", U32), ("show", w.WORD),
                ("reserved_size", w.WORD), ("reserved_bytes", P), ("stdin", P), ("stdout", P), ("stderr", P)]


class Information(c.Structure):
    _fields_ = [("process", P), ("thread", P), ("pid", U32), ("tid", U32)]


class SecurityAttributes(c.Structure):
    _fields_ = [("size", U32), ("descriptor", P), ("inherit", w.BOOL)]


class BasicLimits(c.Structure):
    _fields_ = [("process_time", c.c_int64), ("job_time", c.c_int64), ("flags", U32),
                ("minimum", SIZE), ("maximum", SIZE), ("active", U32), ("affinity", SIZE),
                ("priority", U32), ("scheduling", U32)]


class JobLimits(c.Structure):
    _fields_ = [("basic", BasicLimits), ("io", U64 * 6), ("process_memory", SIZE),
                ("job_memory", SIZE), ("peak_process", SIZE), ("peak_job", SIZE)]


close = api(kernel, "CloseHandle", w.BOOL, P)
free = api(kernel, "LocalFree", P, P)
get_security = api(adv, "GetNamedSecurityInfoW", U32, w.LPWSTR, U32, U32, P, P, P, P, P)
set_security = api(adv, "SetNamedSecurityInfoW", U32, w.LPWSTR, U32, U32, P, P, P, P)


def principal(stack):
    name = "rrq_" + secrets.token_hex(6)
    password = "Aa1!" + secrets.token_urlsafe(32)
    user = User(name, password, 0, 1, None, "RRFlow disposable qualification", 0x10201, None)
    checked(api(net, "NetUserAdd", U32, w.LPCWSTR, U32, P, P)(None, 1, c.byref(user), None), "create qualification principal")
    delete = api(net, "NetUserDel", U32, w.LPCWSTR, w.LPCWSTR)
    stack.callback(lambda: checked(delete(None, name), "delete qualification principal"))
    sid_length, domain_length, kind = U32(), U32(), U32()
    lookup = api(adv, "LookupAccountNameW", w.BOOL, w.LPCWSTR, w.LPCWSTR, P, P, w.LPWSTR, P, P)
    lookup(None, name, None, c.byref(sid_length), None, c.byref(domain_length), c.byref(kind))
    require(0 < sid_length.value < 1024 and domain_length.value < 1024, "unbounded principal identity")
    sid = c.create_string_buffer(sid_length.value)
    domain = c.create_unicode_buffer(domain_length.value)
    boolean(lookup(None, name, sid, c.byref(sid_length), domain, c.byref(domain_length), c.byref(kind)), "resolve qualification principal")
    text = P()
    boolean(api(adv, "ConvertSidToStringSidW", w.BOOL, P, P)(sid, c.byref(text)), "format principal")
    try:
        sid_text = c.wstring_at(text)
    finally:
        free(text)
    # Resolve the built-in Users group by SID, not the runner's display language.
    users = P()
    boolean(api(adv, "ConvertStringSidToSidW", w.BOOL, w.LPCWSTR, P)("S-1-5-32-545", c.byref(users)), "resolve standard user group")
    try:
        group_name, group_domain = c.create_unicode_buffer(256), c.create_unicode_buffer(256)
        name_length, domain_length = U32(256), U32(256)
        boolean(api(adv, "LookupAccountSidW", w.BOOL, w.LPCWSTR, P, w.LPWSTR, P, w.LPWSTR, P, P)(
            None, users, group_name, c.byref(name_length), group_domain, c.byref(domain_length), c.byref(kind)), "resolve standard group name")
        member = c.cast(sid, P)
        status = api(net, "NetLocalGroupAddMembers", U32, w.LPCWSTR, w.LPCWSTR, U32, P, U32)(None, group_name, 0, c.byref(member), 1)
        require(status in {0, 1378}, f"standard group membership failed with Windows status {status}")
    finally:
        free(users)
    token = P()
    boolean(api(adv, "LogonUserW", w.BOOL, w.LPCWSTR, w.LPCWSTR, w.LPCWSTR, U32, U32, P)(
        name, ".", password, 2, 0, c.byref(token)), "log on qualification principal")
    stack.callback(close, token)
    # Root-only ACL masks are sound only when traversal bypass is removed.
    privilege = Privilege()
    boolean(api(adv, "LookupPrivilegeValueW", w.BOOL, w.LPCWSTR, w.LPCWSTR, P)(
        None, "SeChangeNotifyPrivilege", c.byref(privilege.luid)), "resolve traversal privilege")
    restricted = P()
    boolean(api(adv, "CreateRestrictedToken", w.BOOL, P, U32, U32, P, U32, P, U32, P, P)(
        token, 0, 0, None, 1, c.byref(privilege), 0, None, c.byref(restricted)), "remove traversal bypass")
    stack.callback(close, restricted)
    return sid, sid_text, restricted


def access(stack, path, sid, deny):
    original, original_acl = P(), P()
    checked(get_security(str(path), 1, 4, None, None, c.byref(original_acl), None, c.byref(original)), "read fixture ACL")
    stack.callback(free, original)
    entry = Access(0x1F01FF, 3 if deny else 1, 0 if deny else 3, Trustee(None, 0, 0, 1, c.cast(sid, P)))
    changed = P()
    checked(api(adv, "SetEntriesInAclW", U32, U32, P, P, P)(1, c.byref(entry), original_acl, c.byref(changed)), "compose fixture ACL")
    try:
        checked(set_security(str(path), 1, 4, None, None, changed, None), "apply fixture ACL")
    finally:
        free(changed)
    stack.callback(lambda: checked(set_security(str(path), 1, 4, None, None, original_acl, None), "restore fixture ACL"))


def outbound_policy(stack, sid_text):
    require(c.sizeof(Filter) == 200 and c.sizeof(Session) == 72 and c.sizeof(Condition) == 40,
            "unsupported Windows WFP ABI")
    session = Session()
    session.flags = 1  # FWPM_SESSION_FLAG_DYNAMIC: objects die with this session.
    session.display.name = "RRFlow disposable qualification"
    engine = P()
    checked(api(wfp, "FwpmEngineOpen0", U32, w.LPCWSTR, U32, P, P, P)(None, 10, None, c.byref(session), c.byref(engine)), "open dynamic WFP session")
    engine_close = api(wfp, "FwpmEngineClose0", U32, P)
    stack.callback(lambda: checked(engine_close(engine), "close dynamic WFP session"))
    descriptor, length = P(), U32()
    boolean(api(adv, "ConvertStringSecurityDescriptorToSecurityDescriptorW", w.BOOL, w.LPCWSTR, U32, P, P)(
        f"D:(A;;0x1;;;{sid_text})", 1, c.byref(descriptor), c.byref(length)), "encode principal filter")
    stack.callback(free, descriptor)
    blob = Blob(length.value, descriptor)
    conditions = (Condition * 2)()
    conditions[0].field = Guid.value("af043a0a-b34d-4f86-979c-c90371af6e66")
    conditions[0].value.type = 14  # FWP_SECURITY_DESCRIPTOR_TYPE
    conditions[0].value.value.pointer = c.cast(c.pointer(blob), P)
    conditions[1].field = Guid.value("632ce23b-5167-435c-86d7-e903684aa80c")
    conditions[1].match = 8  # FWP_MATCH_FLAGS_NONE_SET
    conditions[1].value.type = 3  # FWP_UINT32
    conditions[1].value.value.number = 1  # FWP_CONDITION_FLAG_IS_LOOPBACK
    add = api(wfp, "FwpmFilterAdd0", U32, P, P, P, P)
    for layer in ["c38d57d1-05a7-4c33-904f-7fbceee60e82", "4a72393b-319f-44bc-84c3-ba54dcb3b6b4"]:
        rule = Filter()
        rule.display.name = "RRFlow qualification principal outbound denial"
        rule.layer = Guid.value(layer)
        rule.count, rule.conditions, rule.action.type = 2, conditions, 0x1001
        checked(add(engine, c.byref(rule), None, None), "add scoped outbound filter")


def private_desktop(stack, sid_text):
    """Own every GUI namespace object without changing a shared desktop ACL."""
    descriptor = P()
    boolean(api(adv, "ConvertStringSecurityDescriptorToSecurityDescriptorW", w.BOOL, w.LPCWSTR, U32, P, P)(
        f"D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;{sid_text})", 1, c.byref(descriptor), None), "encode private desktop security")
    stack.callback(free, descriptor)
    attributes = SecurityAttributes(c.sizeof(SecurityAttributes), descriptor, False)
    old_station = api(user32, "GetProcessWindowStation", P)()
    thread_id = api(kernel, "GetCurrentThreadId", U32)()
    old_desktop = api(user32, "GetThreadDesktop", P, U32)(thread_id)
    boolean(old_station and old_desktop, "retain parent desktop context")
    set_station = api(user32, "SetProcessWindowStation", w.BOOL, P)
    name = "rrflow_" + secrets.token_hex(12)
    try:
        station = api(user32, "CreateWindowStationW", P, w.LPCWSTR, U32, U32, P)(
            name, 1, 0x000F037F, c.byref(attributes))  # create-only, standard + station rights
        boolean(station, "create owned noninteractive window station")
        stack.callback(lambda: boolean(api(user32, "CloseWindowStation", w.BOOL, P)(station), "close owned window station"))
        boolean(set_station(station), "select owned station for desktop creation")
        desktop = api(user32, "CreateDesktopW", P, w.LPCWSTR, P, P, U32, U32, P)(
            "default", None, None, 0, 0x000F01FF, c.byref(attributes))
        boolean(desktop, "create owned noninteractive desktop")
        stack.callback(lambda: boolean(api(user32, "CloseDesktop", w.BOOL, P)(desktop), "close owned desktop"))
    finally:
        boolean(set_station(old_station), "restore parent window station")
        boolean(api(user32, "SetThreadDesktop", w.BOOL, P)(old_desktop), "restore parent thread desktop")
    return name + "\\default"


def probe_runtime_access(token, stage):
    """Check only known qualification dependencies under the actual token."""
    paths = [Path(sys.executable), Path(encodings.__file__), stage / "runner.py"]
    module_name = api(kernel, "GetModuleFileNameW", U32, P, w.LPWSTR, U32)
    for handle in [sys.dllhandle, kernel._handle, adv._handle, user32._handle]:
        buffer = c.create_unicode_buffer(32768)
        length = module_name(handle, buffer, len(buffer))
        require(0 < length < len(buffer), "qualification runtime module path unavailable")
        paths.append(Path(buffer.value))
    boolean(api(adv, "ImpersonateLoggedOnUser", w.BOOL, P)(token), "inspect restricted runtime access")
    try:
        for path in paths:
            try:
                with path.open("rb") as source:
                    require(bool(source.read(64)), "qualification runtime dependency is empty")
            except OSError as error:
                raise RuntimeError(f"qualification runtime access denied for {path.name}") from error
    finally:
        boolean(api(adv, "RevertToSelf", w.BOOL)(), "restore qualification caller token")


def run_isolated(stage: Path, masks: list[Path]):
    require(sys.platform == "win32" and os.environ.get("GITHUB_ACTIONS") == "true"
            and os.environ.get("RUNNER_ENVIRONMENT") == "github-hosted",
            "Windows account and native policy qualification requires an ephemeral hosted CI runner")
    require(stage.is_relative_to(Path(os.environ["PUBLIC"]).resolve()) and stage.name.startswith("rrflow-offline-"),
            "qualification stage must be an owned public-parent fixture")
    with ExitStack() as stack:
        sid, sid_text, token = principal(stack)
        desktop = private_desktop(stack, sid_text)
        access(stack, stage, sid, False)
        for mask in masks:
            access(stack, mask, sid, True)
        outbound_policy(stack, sid_text)
        probe_runtime_access(token, stage)
        try:
            launch(token, stage, "--windows-inside", 600, desktop)
        finally:
            # The same principal removes only its owned home/temp fixtures after
            # the first job has reaped all descendants. No recursive host ACL repair.
            launch(token, stage, "--cleanup-owned", 60, desktop)


def launch(token, stage, mode, timeout, desktop):
    with ExitStack() as stack:
        job = api(kernel, "CreateJobObjectW", P, P, w.LPCWSTR)(None, None)
        boolean(job, "create owned qualification job")
        stack.callback(quiesce_job, job)  # reap before policy, ACL and principal cleanup
        limits = JobLimits()
        limits.basic.flags = 0x2000  # JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        boolean(api(kernel, "SetInformationJobObject", w.BOOL, P, U32, P, U32)(
            job, 9, c.byref(limits), c.sizeof(limits)), "bound owned job lifetime")
        environment = {"SystemRoot": os.environ["SystemRoot"], "WINDIR": os.environ["SystemRoot"],
                       "PUBLIC": os.environ["PUBLIC"],
                       "TEMP": str(stage / "temporary"), "TMP": str(stage / "temporary"),
                       "USERPROFILE": str(stage / "home"), "HOME": str(stage / "home"),
                       "PATH": os.environ["SystemRoot"] + "\\System32"}
        block = c.create_unicode_buffer("\0".join(f"{key}={value}" for key, value in sorted(environment.items())) + "\0\0")
        command = c.create_unicode_buffer(subprocess.list2cmdline([sys.executable, str(stage / "runner.py"), mode, str(stage)]))
        startup, information = Startup(), Information()
        startup.size = c.sizeof(startup)
        startup.desktop = desktop
        create = api(adv, "CreateProcessWithTokenW", w.BOOL, P, U32, w.LPCWSTR, w.LPWSTR, U32, P, w.LPCWSTR, P, P)
        boolean(create(token, 0, sys.executable, command, 0x08000404, block, str(stage), c.byref(startup), c.byref(information)), "create suspended qualification process")
        stack.callback(close, information.process)
        stack.callback(close, information.thread)
        terminate = api(kernel, "TerminateProcess", w.BOOL, P, U32)
        # Even assignment failure must terminate the still-suspended child.
        stack.callback(terminate, information.process, 1)
        boolean(api(kernel, "AssignProcessToJobObject", w.BOOL, P, P)(job, information.process), "assign owned process before execution")
        require(api(kernel, "ResumeThread", U32, P)(information.thread) != 0xFFFFFFFF, "resume qualification process failed")
        wait = api(kernel, "WaitForSingleObject", U32, P, U32)
        deadline = time.monotonic() + timeout
        while wait(information.process, 1000) == 258:
            require(time.monotonic() < deadline, "Windows qualification exceeded its deadline")
        code = U32()
        boolean(api(kernel, "GetExitCodeProcess", w.BOOL, P, P)(information.process, c.byref(code)), "read qualification exit")
        require(code.value == 0, f"Windows isolated {mode} failed with exit status 0x{code.value:08x}")


def quiesce_job(job):
    try:
        boolean(api(kernel, "TerminateJobObject", w.BOOL, P, U32)(job, 1), "terminate owned job")
        # JOBOBJECT_BASIC_ACCOUNTING_INFORMATION: four i64 times, then four u32
        # counters. ActiveProcesses is the third counter, offset 40.
        counters = (U64 * 6)()
        query = api(kernel, "QueryInformationJobObject", w.BOOL, P, U32, P, U32, P)
        deadline = time.monotonic() + 10
        while True:
            boolean(query(job, 1, counters, c.sizeof(counters), None), "inspect owned job cleanup")
            if c.cast(counters, c.POINTER(U32))[10] == 0:
                break
            require(time.monotonic() < deadline, "owned job descendants did not terminate")
            time.sleep(0.01)
    finally:
        close(job)
