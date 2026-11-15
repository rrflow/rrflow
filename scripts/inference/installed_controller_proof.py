"""Actual installed-process proposal/worker proof, never a quality evaluation.

All model/provider/task inputs are supplied files. No generated labels or model
responses. Starts and terminates only owned child processes; retains no secrets.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
from http.client import RemoteDisconnected
import json
import os
from pathlib import Path
import shutil
import subprocess
import time
import tomllib
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen
from urllib.parse import urlparse
from uuid import uuid4


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    inputs = ['build-receipt', 'contract-example', 'profile', 'task-source',
              'secret-file', 'secret-name', 'environment-name']
    for name in inputs:
        parser.add_argument('--' + name)
    parser.add_argument('--output', required=True)
    parser.add_argument('--recover-proof', help='Read an existing attempt and budget; never dispatch or replay.')
    parser.add_argument('--admission-only', action='store_true', help='Exercise installed denials without provider credentials or dispatch.')
    parser.add_argument('--fault-case', choices=['configuration', 'session', 'stop', 'disconnect'])
    parser.add_argument('--worker', action='store_true',
                        help='Execute one configured selected worker through engine authority; retain draft only.')
    parser.add_argument('--worker-interrupt', action='store_true',
                        help='Kill the owned engine after holding its actual worker response; reopen without redispatch.')
    parser.add_argument('--worker-terminal-rejection', action='store_true',
                        help='Require a real length-terminated worker rejection and prove capacity with another real task.')
    parser.add_argument('--capacity-task-source', help='Separate actual task source for the terminal-rejection capacity proof.')
    parser.add_argument('--worker-terminal-resume', action='store_true',
                        help='Resume the same actual task after terminal rejection and admit a fresh selector/worker.')
    parser.add_argument('--resume-profile', help='Supplied profile restoring only the rejected worker output allowance.')
    parser.add_argument('--native-client', help='Use the task_model_call Rust example for admitted model calls.')
    parser.add_argument('--native-client-timeout-ms', type=int)
    args = parser.parse_args()
    recovery = Path(args.recover_proof).resolve() if args.recover_proof else None
    terminal_worker = args.worker_terminal_rejection or args.worker_terminal_resume
    if args.worker_terminal_resume and (not args.worker or args.worker_terminal_rejection
                                       or args.worker_interrupt or args.fault_case or args.native_client
                                       or recovery or args.admission_only):
        parser.error('Terminal resume requires --worker without other proof modes.')
    if bool(args.resume_profile) != args.worker_terminal_resume:
        parser.error('Terminal resume requires exactly one --resume-profile.')
    if args.worker_terminal_rejection and (not args.worker or args.worker_interrupt or args.fault_case
                                          or args.native_client or recovery or args.admission_only):
        parser.error('Terminal rejection requires --worker without interruption, fault, native, recovery or admission-only modes.')
    if bool(args.capacity_task_source) != args.worker_terminal_rejection:
        parser.error('Terminal rejection requires exactly one separate --capacity-task-source.')
    if args.worker_interrupt and (not args.worker or args.native_client or args.fault_case):
        parser.error('Worker interruption requires --worker and the HTTP proof path, without native-client or completion faults.')
    if args.worker and (recovery or args.admission_only):
        parser.error('Worker mode excludes recovery and admission-only modes.')
    if args.native_client and (recovery or args.admission_only or args.fault_case):
        parser.error('Native client mode is a new baseline proof, not recovery, admission-only or a fault case.')
    if bool(args.native_client) != (args.native_client_timeout_ms is not None):
        parser.error('Native client mode requires both the executable and explicit timeout.')
    if args.native_client_timeout_ms is not None and args.native_client_timeout_ms <= 0:
        parser.error('Native client timeout must be positive.')
    if args.fault_case and (recovery or args.admission_only):
        parser.error('A live fault case excludes recovery and admission-only modes.')
    relay = None
    required_inputs = inputs[:4] if args.admission_only else inputs
    if args.admission_only and (recovery or any(getattr(args, name.replace('-', '_')) is not None for name in inputs[4:])):
        parser.error('Admission-only mode excludes recovery and provider credential inputs.')
    if recovery is None and any(getattr(args, name.replace('-', '_')) is None for name in required_inputs):
        parser.error('A new proof requires all build, profile, task and credential inputs.')
    if recovery is not None and any(getattr(args, name.replace('-', '_')) is not None for name in inputs):
        parser.error('Recovery accepts only --recover-proof and --output.')
    out = Path(args.output).resolve()
    out.mkdir(parents=True, exist_ok=False)

    def save(name, value):
        with (out / name).open('x', encoding='utf-8') as stream:
            json.dump(value, stream, indent=2, ensure_ascii=False)
            stream.write('\n')

    def cli(*parts):
        r = subprocess.run([str(binary), '--json', *map(str, parts)],
                           capture_output=True, text=True, encoding='utf-8', timeout=120)
        if r.returncode:
            raise RuntimeError('CLI failed: ' + str(parts[0]) + ': ' + r.stderr[:1000])
        return json.loads(r.stdout)

    receipt_path = recovery / 'build-receipt.json' if recovery else Path(args.build_receipt)
    receipt = json.loads(receipt_path.read_text(encoding='utf-8'))
    build = receipt['report']['build']
    assert build['provenance']['dirty'] is False
    if recovery:
        binary = recovery / Path(receipt['binary']).name
        assert hashlib.sha256(binary.read_bytes()).hexdigest() == build['executable_sha256']
        assert cli('version')['build'] == build
        project = recovery / 'project'
        save('recovery-source.json', {'proof': str(recovery), 'binarySha256': build['executable_sha256'],
             'sourceCommit': build['provenance']['source_commit'], 'providerDispatchAllowed': False})
    else:
        source_binary = Path(receipt['binary'])
        if not source_binary.is_absolute():
            source_binary = receipt_path.resolve().parent / source_binary
        assert hashlib.sha256(source_binary.read_bytes()).hexdigest() == build['executable_sha256']
        binary = out / source_binary.name
        shutil.copyfile(source_binary, binary)
        assert binary.stat().st_nlink == 1
        save('build-receipt.json', receipt)
        assert cli('version')['build'] == build
        if args.native_client:
            source_client = Path(args.native_client).resolve()
            client_digest = hashlib.sha256(source_client.read_bytes()).hexdigest()
            (out / 'native-client').mkdir()
            native_executable = out / 'native-client' / source_client.name
            shutil.copyfile(source_client, native_executable)
            assert native_executable.stat().st_nlink == 1
            assert hashlib.sha256(native_executable.read_bytes()).hexdigest() == client_digest
            save('native-client-identity.json', {'binarySha256': client_digest, 'sourcePath': str(source_client)})
        profile = json.loads(Path(args.profile).read_text(encoding='utf-8'))
        task_source = json.loads(Path(args.task_source).read_text(encoding='utf-8'))
        if args.worker_terminal_resume:
            from installed_task_resume_proof import validate_resume_profiles
            resume_profile = json.loads(Path(args.resume_profile).read_text(encoding='utf-8'))
            validate_resume_profiles(profile, resume_profile)
            save('resume-profile-input.json', resume_profile)
        if args.worker_terminal_rejection:
            capacity_source = json.loads(Path(args.capacity_task_source).read_text(encoding='utf-8'))
            assert capacity_source['url'] != task_source['url'], 'Capacity proof requires a different actual task source'
            assert capacity_source['title'] and capacity_source['body'] and capacity_source['headRefOid']
            save('capacity-task-source.json', capacity_source)
        save('profile-input.json', profile)
        save('task-source.json', task_source)
        policy = profile['selection_policy']
        if args.worker:
            assert profile.get('worker_policy'), 'Worker mode requires explicit operator worker policy'
        binding = next(b for b in profile['bindings'] if b['id'] == policy['controller_binding_id'])
        assert binding.get('credential'), 'Proof requires an explicitly credentialed binding'
        if args.fault_case or args.worker_interrupt or terminal_worker:
            from installed_controller_fault_relay import FaultRelay
            fault_timeout = binding['limits']['timeout_ms'] / 1000
            response_limit = policy['limits']['maximum_response_bytes']
            if args.worker:
                worker_binding = next((entry for entry in profile['worker_policy']['bindings']
                                       if entry['binding_id'] == binding['id']), None)
                assert worker_binding is not None, 'Relayed controller binding needs explicit worker policy'
                response_limit = max(response_limit, worker_binding['maximum_response_bytes'])
            if terminal_worker:
                assert binding['limits']['max_concurrency'] == 1, 'Capacity proof requires a one-call binding limit'
            relay = FaultRelay(binding['endpoint']['url'], out, fault_timeout,
                               binding['limits']['max_input_bytes'], response_limit,
                               4 if args.worker_terminal_resume else 3 if args.worker_terminal_rejection else
                               2 if args.worker or args.fault_case == 'configuration' else 1,
                               args.worker_interrupt or args.fault_case == 'disconnect',
                               hold_call=2 if args.worker else 1)
            binding['endpoint']['url'] = relay.base_url
            save('effective-profile.json', profile)
            if args.worker_terminal_resume:
                next_binding = next(b for b in resume_profile['bindings'] if b['id'] == binding['id'])
                next_binding['endpoint']['url'] = relay.base_url
                save('effective-resume-profile.json', resume_profile)

        if not args.admission_only:
            mapping = {'format_version': 1, 'entries': [{
                'reference': binding['credential'], 'environment_variable': args.environment_name}]}
            save('credential-mapping.json', mapping)
            secret = next(line.split('=', 1)[1] for line in Path(args.secret_file).read_text(encoding='utf-8').splitlines()
                          if line.startswith(args.secret_name + '='))
        project = out / 'project'
        project.mkdir()
        identity = cli('install', 'identity')['installation_identity']
        plan = cli('install', 'plan', '--project', project, '--mode', 'existing', '--identity', identity)
        save('installation-plan.json', plan)
        save('installation-result.json', cli('install', 'apply', '--project', project, '--mode', 'existing',
             '--plan', out / 'installation-plan.json', '--expect', plan['installation']['plan_sha256']))
    locator = tomllib.loads((project / '.rrflow/config.toml').read_text(encoding='utf-8'))
    credential = json.loads((project / locator['operator_credential']).read_text(encoding='utf-8'))
    instance = locator['identity']['instance_id']
    request_payload = json.loads((recovery / 'proposal-request.json').read_text(encoding='utf-8')) if recovery else None
    accepted = None
    attempt_before_restart = None
    budget_before_restart = None
    worker_request = None
    worker_output = None
    worker_before_restart = None
    capacity_probe = None
    resume_probe = None
    if recovery and (recovery / 'worker-request.json').exists():
        worker_request = json.loads((recovery / 'worker-request.json').read_text(encoding='utf-8'))

    phases = ['recovered'] if recovery else (['missing-credential', 'denial-reopened'] if args.admission_only else ['missing-credential', 'authorized', 'reopened'])
    if args.fault_case:
        phases = ['missing-credential', 'authorized', 'fault-reopened']
    if args.worker:
        phases = ['missing-credential', 'authorized', 'worker-missing-credential',
                  'worker-authorized', ('worker-interrupt-reopened' if args.worker_interrupt else
                                        'worker-fault-reopened' if args.fault_case else 'reopened')]
    if args.worker_terminal_rejection:
        phases = ['missing-credential', 'authorized', 'worker-missing-credential', 'worker-authorized',
                  'worker-terminal-reopened', 'capacity-authorized', 'capacity-reopened']
    if args.worker_terminal_resume:
        phases = ['missing-credential', 'authorized', 'worker-missing-credential', 'worker-authorized',
                  'worker-terminal-reopened', 'resume-authorized', 'resume-reopened']
    for phase in phases:
        log_path = out / (phase + '-server.log')
        env = dict(os.environ)
        command = [str(binary), '--json', 'serve', '--project', str(project), '--bind', '127.0.0.1:0']
        if phase in ['authorized', 'worker-authorized', 'reopened', 'capacity-authorized', 'resume-authorized']:
            env[args.environment_name] = secret
            command += ['--endpoint-credentials', str(out / 'credential-mapping.json')]
        with log_path.open('xb') as log:
            process = subprocess.Popen(command, stdout=log, stderr=log, env=env,
                                       creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
            save(phase + '-owned-process.json', {'pid': process.pid, 'binary': str(binary),
                 'project': str(project), 'binarySha256': build['executable_sha256']})
            try:
                deadline = time.monotonic() + 60
                base = None
                catalogue_path = None
                while base is None:
                    assert process.poll() is None, 'Owned server exited; inspect its log.'
                    for line in log_path.read_text(encoding='utf-8', errors='replace').splitlines():
                        try:
                            announcement = json.loads(line)
                            if announcement.get('status') == 'listening':
                                base = announcement['url']
                                catalogue_path = announcement['endpoint_catalogue_path']
                        except json.JSONDecodeError:
                            pass
                    assert time.monotonic() < deadline, 'Server startup timed out'
                    if base is None:
                        time.sleep(0.2)

                with urlopen(base + catalogue_path, timeout=15) as response:
                    catalogue = json.load(response)
                save(phase + '-catalogue.json', catalogue)
                endpoints = {entry['operation']: entry for entry in catalogue['outcome']['payload']['endpoints']}

                def envelope(payload, mutation):
                    operation = 'controller-proof-' + uuid4().hex
                    return {'protocol': 'rrd', 'protocol_version': 1,
                            'context': {'request_id': operation, 'operation_id': operation,
                                        'idempotency_key': operation if mutation else None, 'deadline_unix_ms': None},
                            'resource': {'segments': [{'kind': 'instance', 'id': instance}]}, 'payload': payload}

                def http(operation, payload, headers=None):
                    descriptor = endpoints[operation]
                    body = envelope(payload, descriptor['mutation'])
                    path = descriptor['path'].replace('{session}', (headers or {}).get('X-RRD-Session', ''))
                    req = Request(base + path, method=descriptor['method'].upper(),
                                  data=json.dumps(body).encode('utf-8'),
                                  headers={'Content-Type': 'application/json', **(headers or {})})
                    try:
                        with urlopen(req, timeout=90) as response:
                            return response.status, json.load(response), body
                    except HTTPError as error:
                        return error.code, json.load(error), body

                def session_headers():
                    status, response, _ = http('session-create', {'limits': {
                        'idle_timeout_ms': 120000, 'absolute_timeout_ms': 300000, 'max_open_transactions': 1}},
                        {'Authorization': 'ApiKey ' + credential['credential'], 'X-RRD-Principal': credential['principal_id']})
                    assert status == 200, 'Session admission failed'
                    session = response['outcome']['payload']
                    return {'Authorization': 'Bearer ' + session['token'], 'X-RRD-Session': session['session_id']}

                headers = session_headers()

                def call(name, operation, payload, expected=200, authenticate=True, error_contains=None):
                    descriptor = endpoints[operation]
                    save(phase + '-' + name + '-intent.json', {'operation': operation, 'path': descriptor['path'], 'payload': payload,
                         'mutation': descriptor['mutation'], 'responseObserved': False})
                    status, response, request = http(operation, payload, headers if authenticate else {})
                    save(phase + '-' + name + '.json', {'status': status, 'request': request, 'response': response})
                    assert status == expected, f'{name} returned {status}, expected {expected}; inspect retained response'
                    if error_contains is not None:
                        assert error_contains in response['outcome']['error']['message'], 'Unexpected denial reason'
                    return response['outcome'].get('payload')

                def native_call(name, operation, request_file, payload, result_field):
                    parsed = urlparse(base)
                    assert parsed.scheme == 'http' and parsed.hostname in ['127.0.0.1', '::1']
                    assert parsed.port is not None
                    command = [str(native_executable), operation, str(project), parsed.netloc,
                               str(out / request_file), str(args.native_client_timeout_ms)]
                    prefix = phase + '-' + name
                    save(prefix + '-intent.json', {'operation': operation, 'payload': payload,
                         'nativeClientSha256': client_digest, 'responseObserved': False})
                    try:
                        completed = subprocess.run(command, capture_output=True,
                                                   timeout=args.native_client_timeout_ms / 1000 * 4)
                    except subprocess.TimeoutExpired as error:
                        (out / (prefix + '.stdout.json')).write_bytes(error.stdout or b'')
                        (out / (prefix + '.stderr.log')).write_bytes(error.stderr or b'')
                        save(prefix + '-exit.json', {'exitCode': None, 'timedOut': True})
                        raise
                    (out / (prefix + '.stdout.json')).write_bytes(completed.stdout)
                    (out / (prefix + '.stderr.log')).write_bytes(completed.stderr)
                    save(prefix + '-exit.json', {'exitCode': completed.returncode, 'timedOut': False})
                    assert completed.returncode == 0, 'Native call failed; preserve output and recover without redispatch.'
                    native = json.loads(completed.stdout)
                    assert native['operation'] == operation and native['request'] == payload
                    assert native['sessionCloseError'] is None and native['outcome']['status'] == 'ok'
                    return native['outcome'][result_field]

                if phase == 'recovered':
                    selection = {key: request_payload[key] for key in ['scope', 'task_id', 'attempt_id']}
                    # No proposal/replay operation and no provider credentials in this phase.
                    assert endpoints['task-attempt-read']['mutation'] is False
                    assert endpoints['task-budget-read']['mutation'] is False
                    attempt = call('attempt', 'task-attempt-read', selection)
                    budget = call('budget', 'task-budget-read', {key: selection[key] for key in ['scope', 'task_id']})
                    if worker_request is not None:
                        worker_before_restart = call('worker-attempt', 'task-attempt-read',
                            {key: worker_request[key] for key in ['scope', 'task_id', 'attempt_id']})
                    if (recovery / 'resume-worker-request.json').exists():
                        from installed_task_resume_proof import read_resume_probe
                        recorded = json.loads((recovery / 'result.json').read_text(encoding='utf-8'))
                        resume_probe = read_resume_probe(call, recorded['resumeProbe'])
                        assert budget == resume_probe['budget']
                        assert attempt == recorded['selectorAttempt']
                        assert worker_before_restart == recorded['workerAttempt']
                elif phase in ['worker-interrupt-reopened', 'worker-fault-reopened',
                               'worker-terminal-reopened', 'capacity-reopened', 'resume-reopened']:
                    worker_selection = {key: worker_request[key] for key in ['scope', 'task_id', 'attempt_id']}
                    budget_selection = {key: worker_request[key] for key in ['scope', 'task_id']}
                    assert call('worker-attempt', 'task-attempt-read', worker_selection) == worker_before_restart
                    assert call('budget', 'task-budget-read', budget_selection) == budget_before_restart
                    assert call('selector-attempt', 'task-attempt-read',
                                {key: request_payload[key] for key in ['scope', 'task_id', 'attempt_id']}) == attempt_before_restart
                    assert call('task', 'task-read', {**budget_selection,
                                'max_storage_keys': request_payload['max_storage_keys']})['task'] == task
                    if phase == 'capacity-reopened':
                        assert call('capacity-attempt', 'task-attempt-read',
                                    {key: capacity_request[key] for key in ['scope', 'task_id', 'attempt_id']}) == capacity_probe['attempt']
                        assert call('capacity-budget', 'task-budget-read',
                                    {key: capacity_request[key] for key in ['scope', 'task_id']}) == capacity_probe['budget']
                    if phase == 'resume-reopened':
                        from installed_task_resume_proof import read_resume_probe
                        assert read_resume_probe(call, resume_probe) == resume_probe
                elif phase == 'resume-authorized':
                    from installed_task_resume_proof import execute_resume_probe
                    resume_probe = execute_resume_probe(
                        call, save, out, args.contract_example, relay, resume_profile, configuration,
                        task, request_payload, attempt_before_restart, worker_request,
                        worker_before_restart, budget_before_restart, binding['id'])
                    budget_before_restart = resume_probe['budget']
                elif phase == 'capacity-authorized':
                    original_budget = {key: worker_request[key] for key in ['scope', 'task_id']}
                    call('original-worker-replay', 'task-worker-execute', worker_request, 412,
                         error_contains='ReasoningAttemptRecorded')
                    assert relay.calls == 2, 'Replay must not redispatch'
                    assert call('original-budget', 'task-budget-read', original_budget) == budget_before_restart
                    goal = capacity_source['body'].encode('utf-8')[:4096].decode('utf-8', errors='ignore')
                    capacity_task = call('open-capacity-task', 'task-open', {'scope': 'instance:' + instance,
                                         'title': capacity_source['title'], 'goal': goal})['task']
                    assert capacity_task['task_id'] != task['task_id']
                    save('capacity-task-record.json', capacity_task)
                    save('capacity-task-context-projection.json', {'sourceUrl': capacity_source['url'],
                         'originalBodySha256': hashlib.sha256(capacity_source['body'].encode('utf-8')).hexdigest(),
                         'projectedBodyBytes': len(goal.encode('utf-8')), 'truncated': goal != capacity_source['body'],
                         'use': 'separate real-task capacity proof; no retry or training label'})
                    generated = subprocess.run([args.contract_example, 'proposal-request',
                        str(out / 'capacity-task-record.json'), str(out / 'configuration.json'),
                        'actual-capacity-proposal-' + uuid4().hex], capture_output=True, text=True,
                        encoding='utf-8', timeout=30, check=True)
                    capacity_request = json.loads(generated.stdout)
                    save('capacity-proposal-request.json', capacity_request)
                    proposal = call('capacity-proposal', 'task-endpoint-propose', capacity_request)
                    assert proposal['controller_binding_id'] == binding['id']
                    assert relay.calls == 3 and relay.error is None
                    capacity_attempt = call('capacity-attempt', 'task-attempt-read',
                                            {key: capacity_request[key] for key in ['scope', 'task_id', 'attempt_id']})
                    assert capacity_attempt['attempt']['status'] == 'accepted'
                    assert capacity_attempt['attempt']['endpoint_proposal'] == proposal
                    capacity_budget = call('capacity-budget', 'task-budget-read',
                                           {key: capacity_request[key] for key in ['scope', 'task_id']})
                    assert capacity_budget['budget']['reserved_steps'] == 1
                    assert capacity_budget['budget']['active_attempt'] is None
                    assert capacity_budget['budget']['execution_may_continue'] is False
                    assert capacity_budget['budget']['stopped'] is None
                    capacity_probe = {'proposal': proposal, 'attempt': capacity_attempt, 'budget': capacity_budget}
                    assert call('original-budget-after-capacity', 'task-budget-read', original_budget) == budget_before_restart
                    assert call('original-worker-after-capacity', 'task-attempt-read',
                                {key: worker_request[key] for key in ['scope', 'task_id', 'attempt_id']}) == worker_before_restart
                elif phase in ['worker-missing-credential', 'worker-authorized']:
                    assert worker_request is not None
                    worker_selection = {key: worker_request[key] for key in ['scope', 'task_id', 'attempt_id']}
                    budget_selection = {key: worker_request[key] for key in ['scope', 'task_id']}
                    if phase == 'worker-missing-credential':
                        call('worker-missing-credential', 'task-worker-execute', worker_request, 412,
                             error_contains='configured endpoint credential is unavailable')
                        assert call('budget', 'task-budget-read', budget_selection) == budget_before_restart
                        missing = call('worker-attempt', 'task-attempt-read', worker_selection)
                        assert missing['attempt'] is None
                    elif terminal_worker:
                        with ThreadPoolExecutor(max_workers=1) as executor:
                            pending = executor.submit(call, 'terminal-worker', 'task-worker-execute', worker_request,
                                                      412, True, 'chat response is incomplete or requests unsupported effects')
                            try:
                                assert relay.ready.wait(fault_timeout), 'No actual terminal worker response observed'
                                assert relay.error is None and relay.calls == 2
                                actual = json.loads((out / 'relay-2-provider-response.json').read_bytes())
                                assert actual['model'] == binding['service_id'] and len(actual['choices']) == 1
                                choice = actual['choices'][0]
                                assert choice['finish_reason'] == 'length', 'Provider did not produce the requested fault condition'
                                assert choice['message']['role'] == 'assistant'
                                assert not choice['message'].get('tool_calls') and not choice['message'].get('function_call')
                                reserved = call('reserved-worker', 'task-attempt-read', worker_selection)
                                assert reserved['attempt']['status'] == 'reserved'
                                active = call('reserved-budget', 'task-budget-read', budget_selection)
                                assert active['budget']['active_attempt'] == worker_request['attempt_id']
                                assert active['budget']['reserved_steps'] == 2 and active['budget']['execution_may_continue']
                                if args.worker_terminal_resume:
                                    call('resume-inflight-denied', 'task-budget-resume',
                                         {**budget_selection, 'expected_budget_sha256': active['budget_sha256']},
                                         412, error_contains='UnresolvedAttempt')
                                    assert call('after-denied-resume-budget', 'task-budget-read', budget_selection) == active
                            finally:
                                relay.release.set()
                            pending.result(timeout=90)
                        worker_before_restart = call('terminal-worker-attempt', 'task-attempt-read', worker_selection)
                        record = worker_before_restart['attempt']
                        assert record['status'] == 'stopped' and record.get('worker_output') is None
                        assert record['outcome_sha256'] is None
                        budget = call('terminal-worker-budget', 'task-budget-read', budget_selection)
                        assert budget['budget']['reserved_steps'] == 2 and budget['budget']['active_attempt'] is None
                        assert budget['budget']['execution_may_continue'] is False
                        assert budget['budget']['stopped'] == 'attempt_failed'
                        assert budget['budget']['deadline_unix_ms'] == budget_before_restart['budget']['deadline_unix_ms']
                        assert budget['budget']['charged_elapsed_ms'] >= budget_before_restart['budget']['charged_elapsed_ms']
                        assert call('selector-after-rejection', 'task-attempt-read',
                                    {key: request_payload[key] for key in ['scope', 'task_id', 'attempt_id']}) == attempt_before_restart
                        assert call('task-after-rejection', 'task-read', {**budget_selection,
                                    'max_storage_keys': request_payload['max_storage_keys']})['task'] == task
                        budget_before_restart = budget
                    elif args.fault_case:
                        reasons = {'configuration': 'InputsChanged', 'session': 'AuthorizationChanged',
                                   'stop': 'chat cancelled', 'disconnect': 'remote outcome unknown'}
                        with ThreadPoolExecutor(max_workers=1) as executor:
                            pending = executor.submit(call, 'fault-worker', 'task-worker-execute',
                                                      worker_request, 412, True, reasons[args.fault_case])
                            try:
                                assert relay.ready.wait(fault_timeout), 'Actual worker response was not observed at relay'
                                assert relay.error is None, 'Provider/relay failed before worker completion fault'
                                assert relay.calls == 2, 'Fault requires exactly the actual selector and worker calls'
                                reserved = call('reserved-worker', 'task-attempt-read', worker_selection)
                                assert reserved['attempt']['status'] == 'reserved'
                                assert reserved['attempt'].get('worker_output') is None
                                active = call('reserved-budget', 'task-budget-read', budget_selection)
                                assert active['budget']['reserved_steps'] == 2
                                assert active['budget']['active_attempt'] == worker_request['attempt_id']
                                assert active['budget']['execution_may_continue'] is True
                                assert active['budget']['deadline_unix_ms'] == budget_before_restart['budget']['deadline_unix_ms']
                                if args.fault_case == 'configuration':
                                    plan = call('replace-preview', 'endpoint-configuration-preview',
                                                {'expected_revision': configuration['revision'], 'profile': profile})
                                    configuration = call('replace-apply', 'endpoint-configuration-apply',
                                                         {'plan': plan, 'approved_plan_sha256': plan['plan_sha256']})
                                elif args.fault_case == 'session':
                                    call('close-inflight-session', 'session-close', {})
                                elif args.fault_case == 'stop':
                                    call('stop-inflight', 'task-budget-stop',
                                         {**budget_selection, 'expected_budget_sha256': active['budget_sha256']})
                            finally:
                                relay.release.set()
                            pending.result(timeout=90)
                        if args.fault_case == 'session':
                            headers = session_headers()
                        worker_before_restart = call('fault-worker-attempt', 'task-attempt-read', worker_selection)
                        record = worker_before_restart['attempt']
                        assert record.get('worker_output') is None and record['outcome_sha256'] is None
                        budget = call('fault-worker-budget', 'task-budget-read', budget_selection)
                        assert budget['budget']['reserved_steps'] == 2
                        assert budget['budget']['active_attempt'] is None
                        assert budget['budget']['deadline_unix_ms'] == budget_before_restart['budget']['deadline_unix_ms']
                        assert budget['budget']['charged_elapsed_ms'] >= budget_before_restart['budget']['charged_elapsed_ms']
                        if args.fault_case in ['configuration', 'session']:
                            assert record['status'] == 'invalidated'
                            assert budget['budget']['execution_may_continue'] is False
                            assert budget['budget']['stopped'] is None
                        else:
                            assert record['status'] == 'stopped'
                            assert budget['budget']['execution_may_continue'] is True
                            assert budget['budget']['stopped'] == ('cancelled' if args.fault_case == 'stop' else 'attempt_failed')
                        assert call('selector-after-fault', 'task-attempt-read',
                                    {key: request_payload[key] for key in ['scope', 'task_id', 'attempt_id']}) == attempt_before_restart
                        assert call('task-after-fault', 'task-read', {**budget_selection,
                                    'max_storage_keys': request_payload['max_storage_keys']})['task'] == task
                        budget_before_restart = budget
                    elif args.worker_interrupt:
                        with ThreadPoolExecutor(max_workers=1) as executor:
                            pending = executor.submit(call, 'interrupted-worker', 'task-worker-execute', worker_request)
                            try:
                                assert relay.ready.wait(fault_timeout), 'Actual worker response was not observed at the relay'
                                assert relay.error is None, 'Provider/relay failed before interruption'
                                assert relay.calls == 2, 'Interruption must follow exactly the selector and worker calls'
                                worker_before_restart = call('reserved-worker', 'task-attempt-read', worker_selection)
                                assert worker_before_restart['attempt']['status'] == 'reserved'
                                assert worker_before_restart['attempt'].get('worker_output') is None
                                budget = call('reserved-budget', 'task-budget-read', budget_selection)
                                assert budget['budget']['reserved_steps'] == 2
                                assert budget['budget']['active_attempt'] == worker_request['attempt_id']
                                assert budget['budget']['execution_may_continue'] is True
                                assert budget['budget']['stopped'] is None
                                assert budget['budget']['deadline_unix_ms'] == budget_before_restart['budget']['deadline_unix_ms']
                                assert call('task-before-interruption', 'task-read', {**budget_selection,
                                            'max_storage_keys': request_payload['max_storage_keys']})['task'] == task
                                budget_before_restart = budget
                                process.kill()
                                process.wait(timeout=15)
                                save('worker-process-interruption.json', {'pid': process.pid,
                                     'exitCode': process.returncode, 'binarySha256': build['executable_sha256'],
                                     'actualWorkerResponseHeld': True, 'providerTerminationClaimed': False})
                            finally:
                                relay.release.set()
                            try:
                                pending.result(timeout=90)
                            except (RemoteDisconnected, URLError, ConnectionResetError, ConnectionAbortedError) as error:
                                save('worker-caller-interruption.json', {'errorType': type(error).__name__,
                                     'acceptedResponseObserved': False, 'redispatchRequested': False})
                            else:
                                raise AssertionError('Interrupted worker unexpectedly returned an accepted response')
                    else:
                        worker_output = (native_call('native-worker', 'task-worker-execute',
                                                     'worker-request.json', worker_request, 'worker')
                                         if args.native_client else
                                         call('worker', 'task-worker-execute', worker_request))
                        for field, expected in {
                            'scope': worker_request['scope'], 'task_id': worker_request['task_id'],
                            'attempt_id': worker_request['attempt_id'],
                            'selection_attempt_id': worker_request['selection_attempt_id'],
                            'selection_sha256': worker_request['expected_selection_sha256'],
                            'task_sha256': worker_request['expected_task_sha256'],
                            'configuration_sha256': worker_request['expected_configuration_sha256'],
                            'binding_id': accepted['choice']['binding_id'],
                        }.items():
                            assert worker_output[field] == expected, 'Worker identity mismatch: ' + field
                        assert hashlib.sha256(worker_output['content'].encode('utf-8')).hexdigest() == worker_output['content_sha256']
                        worker_before_restart = call('worker-attempt', 'task-attempt-read', worker_selection)
                        assert worker_before_restart['attempt']['status'] == 'accepted'
                        assert worker_before_restart['attempt']['worker_output'] == worker_output
                        budget = call('worker-budget', 'task-budget-read', budget_selection)
                        assert budget['budget']['reserved_steps'] == 2
                        assert budget['budget']['active_attempt'] is None
                        assert budget['budget']['execution_may_continue'] is False
                        assert budget['budget']['deadline_unix_ms'] == budget_before_restart['budget']['deadline_unix_ms']
                        call('worker-replay', 'task-worker-execute', worker_request, 412,
                             error_contains='ReasoningAttemptRecorded')
                        assert call('after-replay-budget', 'task-budget-read', budget_selection) == budget
                        task_view = call('task', 'task-read', {**budget_selection,
                                         'max_storage_keys': request_payload['max_storage_keys']})
                        assert task_view['task'] == task, 'Worker text must not complete or alter the task'
                        budget_before_restart = budget
                elif phase == 'denial-reopened':
                    budget = call('budget', 'task-budget-read', {key: request_payload[key] for key in ['scope', 'task_id']})
                    assert budget['budget'] is None
                    readback = call('configuration', 'endpoint-configuration-read', {})
                    assert readback == configuration
                    call('current-request', 'task-endpoint-propose', request_payload, 412,
                         error_contains='configured endpoint credential is unavailable')
                    assert call('after-denial-budget', 'task-budget-read',
                                {key: request_payload[key] for key in ['scope', 'task_id']})['budget'] is None
                elif phase == 'missing-credential':
                    preview = call('preview', 'endpoint-configuration-preview', {'expected_revision': 0, 'profile': profile})
                    configuration = call('apply', 'endpoint-configuration-apply',
                                         {'plan': preview, 'approved_plan_sha256': preview['plan_sha256']})
                    # Exact bounded prefix of the actual PR body, with truncation
                    # explicitly recorded; no invented task or correctness label.
                    goal = task_source['body'].encode('utf-8')[:4096].decode('utf-8', errors='ignore')
                    task = call('open-task', 'task-open', {'scope': 'instance:' + instance,
                                'title': task_source['title'], 'goal': goal})['task']
                    save('task-context-projection.json', {'sourceUrl': task_source['url'],
                         'originalBodySha256': hashlib.sha256(task_source['body'].encode('utf-8')).hexdigest(),
                         'projectedBodyBytes': len(goal.encode('utf-8')), 'truncated': goal != task_source['body'],
                         'use': 'transport/admission proof only; no training or quality label'})
                    save('task-record.json', task)
                    save('configuration.json', configuration)
                    result = subprocess.run([args.contract_example, 'proposal-request', str(out / 'task-record.json'),
                         str(out / 'configuration.json'), 'actual-proposal-' + uuid4().hex], capture_output=True,
                         text=True, encoding='utf-8', timeout=30, check=True)
                    request_payload = json.loads(result.stdout)
                    save('proposal-request.json', request_payload)
                    call('unauthenticated', 'task-endpoint-propose', request_payload, 401, False)
                    call('missing-credential', 'task-endpoint-propose', request_payload, 412)
                    budget = call('budget', 'task-budget-read', {'scope': task['scope'], 'task_id': task['task_id']})
                    assert budget['budget'] is None
                    if args.admission_only:
                        def replace_profile(name, replacement):
                            nonlocal configuration
                            plan = call(name + '-preview', 'endpoint-configuration-preview',
                                        {'expected_revision': configuration['revision'], 'profile': replacement})
                            configuration = call(name + '-apply', 'endpoint-configuration-apply',
                                                 {'plan': plan, 'approved_plan_sha256': plan['plan_sha256']})
                            return configuration

                        def refresh_request(name):
                            save(name + '-configuration.json', configuration)
                            result = subprocess.run([args.contract_example, 'proposal-request',
                                str(out / 'task-record.json'), str(out / (name + '-configuration.json')),
                                request_payload['attempt_id']], capture_output=True, text=True,
                                encoding='utf-8', timeout=30, check=True)
                            return json.loads(result.stdout)

                        # These are actual operator policy changes on this isolated engram,
                        # not altered provider observations or invented training labels.
                        restricted = json.loads(json.dumps(profile))
                        restricted['selection_policy']['allow_network'] = False
                        replace_profile('network-denied', restricted)
                        call('network-denied', 'task-endpoint-propose', refresh_request('network-denied'),
                             403, error_contains='PermissionDenied')
                        replace_profile('restored', profile)
                        call('stale-configuration', 'task-endpoint-propose', request_payload,
                             412, error_contains='InputsChanged')
                        request_payload = refresh_request('restored')
                        call('restored-configuration', 'task-endpoint-propose', request_payload,
                             412, error_contains='configured endpoint credential is unavailable')
                        replace_profile('revoked', None)
                        call('revoked-configuration', 'task-endpoint-propose', request_payload,
                             400, error_contains='endpoint profile is absent')
                        replace_profile('reactivated', profile)
                        request_payload = refresh_request('reactivated')
                        save('current-proposal-request.json', request_payload)
                        assert call('denials-budget', 'task-budget-read',
                                    {key: request_payload[key] for key in ['scope', 'task_id']})['budget'] is None
                        call('close-session', 'session-close', {})
                        call('closed-session', 'task-endpoint-propose', request_payload,
                             412, error_contains='SessionExpired')
                else:
                    assert request_payload is not None
                    selection = {key: request_payload[key] for key in ['scope', 'task_id', 'attempt_id']}
                    if args.fault_case and not args.worker:
                        if phase == 'authorized':
                            reasons = {'configuration': 'InputsChanged', 'session': 'AuthorizationChanged',
                                       'stop': 'chat cancelled', 'disconnect': 'remote outcome unknown'}
                            with ThreadPoolExecutor(max_workers=1) as executor:
                                pending = executor.submit(call, 'fault-proposal', 'task-endpoint-propose',
                                                          request_payload, 412, True, reasons[args.fault_case])
                                try:
                                    assert relay.ready.wait(fault_timeout), 'No actual provider response at relay'
                                    assert relay.error is None, 'Provider/relay failed before controlled fault'
                                    if args.fault_case == 'configuration':
                                        plan = call('replace-preview', 'endpoint-configuration-preview',
                                                    {'expected_revision': configuration['revision'], 'profile': profile})
                                        configuration = call('replace-apply', 'endpoint-configuration-apply',
                                                             {'plan': plan, 'approved_plan_sha256': plan['plan_sha256']})
                                    elif args.fault_case == 'session':
                                        call('close-inflight-session', 'session-close', {})
                                    elif args.fault_case == 'stop':
                                        active = call('active-budget', 'task-budget-read',
                                                      {key: selection[key] for key in ['scope', 'task_id']})
                                        call('stop-inflight', 'task-budget-stop',
                                             {**{key: selection[key] for key in ['scope', 'task_id']},
                                              'expected_budget_sha256': active['budget_sha256']})
                                finally:
                                    relay.release.set()
                                pending.result(timeout=90)
                            if args.fault_case == 'session':
                                headers = session_headers()
                            attempt = call('fault-attempt', 'task-attempt-read', selection)
                            budget = call('fault-budget', 'task-budget-read',
                                          {key: selection[key] for key in ['scope', 'task_id']})
                            assert attempt['attempt'].get('endpoint_proposal') is None
                            assert budget['budget']['reserved_steps'] == 1
                            assert budget['budget']['active_attempt'] is None
                            if args.fault_case in ['configuration', 'session']:
                                assert attempt['attempt']['status'] == 'invalidated'
                                assert budget['budget']['execution_may_continue'] is False
                                assert budget['budget']['stopped'] is None
                            else:
                                assert attempt['attempt']['status'] == 'stopped'
                                assert budget['budget']['execution_may_continue'] is True
                                assert budget['budget']['stopped'] == ('cancelled' if args.fault_case == 'stop' else 'attempt_failed')
                            if args.fault_case == 'configuration':
                                save('changed-configuration.json', configuration)
                                fresh = subprocess.run([args.contract_example, 'proposal-request', str(out / 'task-record.json'),
                                    str(out / 'changed-configuration.json'), 'actual-successor-' + uuid4().hex],
                                    capture_output=True, text=True, encoding='utf-8', timeout=30, check=True)
                                fresh_request = json.loads(fresh.stdout)
                                save('fresh-proposal-request.json', fresh_request)
                                accepted = call('fresh-proposal', 'task-endpoint-propose', fresh_request)
                                next_budget = call('fresh-budget', 'task-budget-read',
                                                   {key: selection[key] for key in ['scope', 'task_id']})
                                assert next_budget['budget']['reserved_steps'] == 2
                                assert next_budget['budget']['deadline_unix_ms'] == budget['budget']['deadline_unix_ms']
                                assert next_budget['budget']['charged_elapsed_ms'] >= budget['budget']['charged_elapsed_ms']
                                budget = next_budget
                            attempt_before_restart, budget_before_restart = attempt, budget
                        else:
                            attempt = call('fault-attempt', 'task-attempt-read', selection)
                            budget = call('fault-budget', 'task-budget-read',
                                          {key: selection[key] for key in ['scope', 'task_id']})
                            assert attempt == attempt_before_restart and budget == budget_before_restart
                            if args.fault_case == 'configuration':
                                fresh_attempt = call('fresh-attempt', 'task-attempt-read',
                                                     {key: fresh_request[key] for key in ['scope', 'task_id', 'attempt_id']})
                                assert fresh_attempt['attempt']['endpoint_proposal'] == accepted
                        continue
                    if phase == 'authorized':
                        if args.native_client:
                            accepted = native_call('native-proposal', 'task-endpoint-propose',
                                                   'proposal-request.json', request_payload, 'proposal')
                        else:
                            accepted = call('proposal', 'task-endpoint-propose', request_payload)
                    else:
                        call('replay', 'task-endpoint-propose', request_payload, 412)
                    attempt = call('attempt', 'task-attempt-read', selection)
                    budget = call('budget', 'task-budget-read', {key: selection[key] for key in ['scope', 'task_id']})
                    assert attempt['attempt']['status'] == 'accepted'
                    assert attempt['attempt']['endpoint_proposal'] == accepted
                    assert budget['budget']['reserved_steps'] == (2 if args.worker and phase == 'reopened' else 1)
                    assert budget['budget']['active_attempt'] is None
                    assert budget['budget']['execution_may_continue'] is False
                    if phase == 'authorized':
                        attempt_before_restart, budget_before_restart = attempt, budget
                        if args.worker:
                            assert accepted['choice']['kind'] == 'select', 'Actual controller abstained; do not fabricate a selection'
                            if args.worker_interrupt or args.fault_case or terminal_worker:
                                assert accepted['choice']['binding_id'] == binding['id'], 'Worker faults require the actual selected binding to be the relayed binding'
                            worker_request = {
                                'scope': request_payload['scope'], 'task_id': request_payload['task_id'],
                                'attempt_id': 'actual-worker-' + uuid4().hex,
                                'selection_attempt_id': request_payload['attempt_id'],
                                'expected_selection_sha256': attempt['attempt']['outcome_sha256'],
                                'expected_task_sha256': request_payload['expected_task_sha256'],
                                'expected_configuration_sha256': request_payload['expected_configuration_sha256'],
                                'max_storage_keys': request_payload['max_storage_keys'], 'context': None,
                            }
                            save('worker-request.json', worker_request)
                            call('worker-unauthenticated', 'task-worker-execute', worker_request, 401, False)
                            assert call('after-worker-denial-budget', 'task-budget-read',
                                        {key: selection[key] for key in ['scope', 'task_id']}) == budget
                    else:
                        assert attempt == attempt_before_restart
                        assert budget == budget_before_restart
                        if args.worker:
                            readback = call('worker-attempt', 'task-attempt-read',
                                {key: worker_request[key] for key in ['scope', 'task_id', 'attempt_id']})
                            assert readback == worker_before_restart
                            call('worker-replay', 'task-worker-execute', worker_request, 412,
                                 error_contains='ReasoningAttemptRecorded')
                            assert call('after-worker-replay-budget', 'task-budget-read',
                                        {key: selection[key] for key in ['scope', 'task_id']}) == budget
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=30)
                save(phase + '-owned-process-stopped.json', {'pid': process.pid, 'exitCode': process.returncode})
    if args.worker_terminal_resume:
        relay.close()
        assert relay.error is None and relay.calls == 4
        save('result.json', {'sourceCommit': build['provenance']['source_commit'],
             'binarySha256': build['executable_sha256'],
             'runnerSha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
             'resumeHelperSha256': hashlib.sha256(Path(__file__).with_name('installed_task_resume_proof.py').read_bytes()).hexdigest(),
             'relaySha256': hashlib.sha256(Path(__file__).with_name('installed_controller_fault_relay.py').read_bytes()).hexdigest(),
             'actualProviderCalls': relay.calls, 'acceptedProposal': accepted,
             'selectorAttempt': attempt_before_restart, 'workerAttempt': worker_before_restart,
             'budget': budget_before_restart, 'resumeProbe': resume_probe,
             'sameTaskResumed': True, 'restartReadbackEqual': True,
             'sameAttemptReplayRejected': True, 'originalDeadlinePreserved': True,
             'spentEffortPreserved': True, 'ownedServersStopped': True,
             'modelQualityQualified': False, 'trainingDataCreated': False,
             'scope': 'actual rejected worker, explicit same-task resume and fresh authorized selection/worker; no project effect or unknown lease clearance'})
        print(json.dumps({'output': str(out), 'status': 'installed same-task terminal recovery proof passed'}))
        return
    if args.worker_terminal_rejection:
        relay.close()
        assert relay.error is None and relay.calls == 3
        save('result.json', {'sourceCommit': build['provenance']['source_commit'],
             'binarySha256': build['executable_sha256'],
             'runnerSha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
             'relaySha256': hashlib.sha256(Path(__file__).with_name('installed_controller_fault_relay.py').read_bytes()).hexdigest(),
             'actualProviderCalls': relay.calls, 'acceptedProposal': accepted,
             'selectorAttempt': attempt_before_restart, 'workerAttempt': worker_before_restart,
             'budget': budget_before_restart, 'capacityProbe': capacity_probe,
             'restartReadbackEqual': True, 'sameAttemptReplayRejected': True,
             'capacityAvailableAfterReopen': True, 'originalBudgetUnchangedAfterOtherTask': True,
             'ownedServersStopped': True, 'modelQualityQualified': False, 'trainingDataCreated': False,
             'scope': 'actual terminal worker rejection and separate real-task capacity proof; no original-task resume or historical lease clearance'})
        print(json.dumps({'output': str(out), 'status': 'installed terminal rejection and capacity proof passed'}))
        return
    if args.worker_interrupt or args.worker and args.fault_case:
        relay.close()
        assert relay.error is None and relay.calls == 2
        save('result.json', {'sourceCommit': build['provenance']['source_commit'],
             'binarySha256': build['executable_sha256'],
             'runnerSha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
             'relaySha256': hashlib.sha256(Path(__file__).with_name('installed_controller_fault_relay.py').read_bytes()).hexdigest(),
             'faultCase': 'process-interruption' if args.worker_interrupt else args.fault_case,
             'actualProviderCalls': relay.calls, 'acceptedProposal': accepted,
             'selectorAttempt': attempt_before_restart, 'workerAttempt': worker_before_restart,
             'budget': budget_before_restart, 'restartReadbackEqual': True,
             'recoveryProviderCredentialsSupplied': False, 'recoveryModelOperationRequested': False,
             'ownedServersStopped': True, 'modelQualityQualified': False, 'trainingDataCreated': False,
             'scope': ('owned engine killed after actual worker response, before engine acceptance; retained uncertainty, no retry or provider termination claim'
                       if args.worker_interrupt else
                       'actual worker response withheld after controlled completion fault; no retry, capacity reconciliation or provider termination claim')})
        print(json.dumps({'output': str(out), 'status': 'installed worker fault and readback passed'}))
        return
    if args.fault_case:
        relay.close()
        assert relay.calls == (2 if args.fault_case == 'configuration' else 1)
        save('result.json', {'sourceCommit': build['provenance']['source_commit'],
             'binarySha256': build['executable_sha256'],
             'runnerSha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
             'relaySha256': hashlib.sha256(Path(__file__).with_name('installed_controller_fault_relay.py').read_bytes()).hexdigest(),
             'faultCase': args.fault_case, 'actualProviderCalls': relay.calls,
             'attempt': attempt_before_restart, 'budget': budget_before_restart,
             'freshProposal': accepted, 'restartReadbackEqual': True,
             'ownedServersStopped': True, 'modelQualityQualified': False, 'trainingDataCreated': False,
             'scope': 'actual provider response held/dropped before engine publication; no provider termination claim'})
        print(json.dumps({'output': str(out), 'status': 'installed live fault and reopen checks passed'}))
        return
    if args.admission_only:
        save('result.json', {'sourceCommit': build['provenance']['source_commit'],
             'binarySha256': build['executable_sha256'],
             'runnerSha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
             'checks': ['unauthenticated', 'missing-credential', 'network-denied',
                        'stale-configuration', 'restored-configuration', 'revoked-configuration',
                        'closed-session', 'configuration-reopen', 'denials-reserve-no-budget'],
             'providerCredentialsSupplied': False, 'ownedServersStopped': True,
             'modelQualityQualified': False, 'trainingDataCreated': False,
             'scope': 'predispatch installed admission only; no in-flight cancellation or completion claim'})
        print(json.dumps({'output': str(out), 'status': 'installed admission denials and reopen passed'}))
        return
    if recovery:
        save('result.json', {'sourceCommit': build['provenance']['source_commit'],
             'binarySha256': build['executable_sha256'], 'proof': str(recovery),
             'runnerSha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
             'attempt': attempt, 'workerAttempt': worker_before_restart,
             'budget': budget, 'providerCallRequested': False,
             **({'resumeProbe': resume_probe} if resume_probe is not None else {}),
             'ownedServersStopped': True, 'modelQualityQualified': False, 'trainingDataCreated': False})
        print(json.dumps({'output': str(out), 'status': 'existing attempt and budget read; no provider call'}))
        return
    save('result.json', {'sourceCommit': build['provenance']['source_commit'],
         'binarySha256': build['executable_sha256'], 'acceptedProposal': accepted,
         'acceptedWorkerDraft': worker_output, 'workerAttempt': worker_before_restart,
         'runnerSha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
         'restartReadbackEqual': True, 'sameAttemptReplayRejected': True, 'ownedServersStopped': True,
         'engineDispatchObserved': True, 'modelQualityQualified': False, 'trainingDataCreated': False,
         'scope': ('actual controller and worker draft admission, missing credentials, restart and replay; '
                   'no project effects, source-edit plan proof or quality labels' if args.worker else
                   'actual experimental task-text controller transport/admission; no worker execution or source-edit plan proof')})
    print(json.dumps({'output': str(out), 'status': 'actual installed proposal/worker/reopen checks passed' if args.worker
                     else 'actual installed proposal/reopen checks passed'}))


if __name__ == '__main__':
    main()
