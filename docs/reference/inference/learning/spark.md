# Spark preparation for SWAG experiments

**Status:** active preparation guide; hardware unverified and no training run recorded
**Coordinate:** `rrflow://rrflow-instance/data/reference/inference/learning/spark`
**Owner:** optional NVIDIA DGX Spark experiment preparation and hardware evidence requirements

This guide assumes NVIDIA DGX Spark. Confirm the actual machine before
applying platform-specific instructions. It is not Apache Spark, an RRFlow
installer, a trained controller, or an engine readiness requirement. The
[controller training contract](controller-training.md) owns data, evaluation
and promotion; [Gate G](../../../roadmap/rrflow-1.0/gate-g.md#controller-learning-packages)
owns delivery evidence.

## Establish the host

Complete NVIDIA's [first-boot setup](https://docs.nvidia.com/dgx/dgx-spark/first-boot.html)
and establish authenticated private-network access. Record the observed host,
architecture, OS, driver, CUDA, available RAM/storage and container runtime.
Do not put credentials or full environment dumps into evidence.

Run these read-only observations on the intended host:

```bash
uname -m
nvidia-smi
nvcc --version
python3 --version
docker ps
free -h
df -h
```

Record errors as unavailable prerequisites; do not substitute this Windows
development checkout's results for Spark observations. Host command success
does not prove GPU access inside a container or training-kernel compatibility.

## Reproduce one supported recipe

Use the [NVIDIA NeMo AutoModel Spark instructions](https://build.nvidia.com/spark/nemo-fine-tune/instructions)
as the initial candidate environment. At review time the guide specifies a
container plus pinned source fixes and Spark-specific memory settings.
Resolve and record the exact container digest, source commit and model
revision before running; do not assume the latest tag remains equivalent.
[NVIDIA's Unsloth playbook](https://build.nvidia.com/spark/unsloth/instructions)
is an alternative to compare after the first environment works, not a second
mandatory stack.

Keep checkpoints, approved datasets and experiment outputs in explicit
operator-owned directories outside source control. Preview mounts, ports,
network access and resource limits. Preserve the recipe's validated memory,
sequence-length and attention settings for the first smoke run. Model access
and license acceptance are prerequisites where applicable; no credential is
embedded in a recipe. No remote installation or download occurs merely by
reading this guide.

Run the chosen guide's short training example. Save the effective recipe,
logs, artifact digests, resource measurements and exit status. Then terminate
the trainer, reload its exported checkpoint in a fresh process, perform
inference, and test interruption/resumption separately. A falling loss or a
checkpoint directory is not sufficient evidence.

## Progress to the controller corpus

Use the bounded dataset and held-out evaluation from the training contract.
Measure a simple policy/classifier and small model candidates before selecting
LoRA, QLoRA or another method. Memory capacity alone does not establish
throughput, supported kernels or a suitable deployed model size.

Keep training and serving resource allocations separate. Record the exported
model's template/tokenizer and evaluate its final quantization and runtime on
the actual intended deployment hardware. An ARM Spark training success does
not qualify Windows, macOS, x86 Linux, CPU inference or hosted concurrency.

The first useful result is a reproducible experiment with a reloadable model
and honest baseline comparison. Installed RRFlow admission, artifact
activation, Connectome controls, signed distribution and rollback need their
own G/I/H/J evidence. No friend or family user needs the training environment
to run a separately qualified deployment bundle.
