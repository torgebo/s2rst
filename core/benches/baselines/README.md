# Benchmark baselines

Snapshots of `cargo bench` output used as reference points for optimization
phases. See `PLAN.md` §D.

## Layout

```
baselines/
  YYYYMMDD-<git-short-sha>/
    rustc.txt            # toolchain fingerprint
    host.txt             # uname -a
    iai_<bench>.txt      # raw iai-callgrind stdout
    wall_<bench>.txt     # raw Criterion stdout (also saves --save-baseline main)
```

Populate via:

```
make baseline
```

Do not edit checked-in snapshots — take a fresh one on a new SHA when you
need an updated reference.
