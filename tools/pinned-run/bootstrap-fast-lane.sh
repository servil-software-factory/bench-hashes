#!/bin/bash
# One-time Cherry native fast-lane setup. Run via the unforced provisioning key
# before bench-lockdown. A bounded setup; caller uses an eight-minute timeout.
set -euo pipefail
export HOME=/root DEBIAN_FRONTEND=noninteractive
exec > /root/fast-lane-bootstrap.log 2>&1
echo "start $(date -u -Is)"
cloud-init status --wait || true
# Cherry's networkd-wait-online can fail after SSH and cloud-init finish;
# record that degraded boot and require the actual provisioning stages complete.
cloud-init status --format json > /root/fast-lane-cloud-init.json || true
python3 -c 'import json; x=json.load(open("/root/fast-lane-cloud-init.json")); assert x["status"] == "done" and not x["errors"], x; print("cloud-init", x["extended_status"])'
# No release-history checkout, cargo-c or source build of nextest: this host
# runs pinned jobs against fresh checkouts and receives a checked nextest binary.
timeout -k 5s 180s apt-get -o DPkg::Lock::Timeout=120 update -q
timeout -k 5s 180s apt-get -o DPkg::Lock::Timeout=120 install -y -q \
    build-essential git curl libssl-dev python3 python3-z3 clang lld llvm \
    pkg-config util-linux numactl
if [ ! -x /root/.cargo/bin/rustup ]; then
    timeout -k 5s 150s bash -c 'curl -fsSL https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.99.1 --profile minimal'
fi
. /root/.cargo/env
timeout -k 5s 120s rustup default 1.99.1
timeout -k 5s 120s rustup component add rustfmt llvm-tools
# No unattended background changes on a quiet measurement machine.
systemctl disable --now tuned unattended-upgrades apt-daily.timer apt-daily-upgrade.timer 2>/dev/null || true
mkdir -p /etc/needrestart/conf.d
printf '%s\n' '$nrconf{restart} = "l";' > /etc/needrestart/conf.d/50-list-only.conf
chmod -x /etc/update-motd.d/* 2>/dev/null || true
for file in /sys/devices/system/cpu/cpu*/cpufreq/energy_performance_preference; do
    [ ! -e "$file" ] || echo performance > "$file" 2>/dev/null || true
done
for file in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do
    [ ! -e "$file" ] || echo performance > "$file" 2>/dev/null || true
done
{
    rustc -V
    cargo -V
    clang --version | head -1
    lscpu | grep -E 'Model name|^CPU\(s\)|Socket|NUMA node\(s\)'
    cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null || true
} > /root/fast-lane-versions.txt
echo "done $(date -u -Is)"
