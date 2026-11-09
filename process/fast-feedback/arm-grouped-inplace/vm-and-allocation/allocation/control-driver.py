from pathlib import Path
import os, subprocess, re, sys
w=Path('/workspace/blake3-servil/tmp/wt-grouped-noalloc'); out=Path(sys.argv[1]);out.mkdir()
test=w/'tests/allocations.rs'; lanes=w/'src/lanes.rs'; t=test.read_text(); l=lanes.read_text()
(out/'allocations.rs').write_text(t);(out/'lanes.rs').write_text(l)
env=dict(os.environ,CARGO_TARGET_DIR='/home/b3-builds/grouped-noalloc',TMPDIR='/home/b3-builds/tmp-arm-last',CC='clang-19',CARGO_BUILD_JOBS='4',CARGO_NET_OFFLINE='true')
def run(name,expression):
    p=subprocess.run(['cargo','nextest','run','--release','--test','allocations','-E',expression],cwd=w,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,timeout=100)
    (out/(name+'.log')).write_text(p.stdout);return p
try:
    diag=t.replace('caller.plus(WORKER_COUNTS.get().since(before))','{ let worker = WORKER_COUNTS.get().since(before); eprintln!("OBSERVED caller={caller:?} workers={worker:?}"); caller.plus(worker) }')
    assert diag!=t;test.write_text(diag)
    p=run('baseline-attribution','test(prepared_multithreaded_cycles_include_the_workers)');assert p.returncode!=0;assert 'workers=Counts { allocations: 0, reallocations: 0, deallocations: 0, bytes: 0 }' in p.stdout;print('BASELINE-ATTRIBUTED-TO-CALLER',flush=True)
    needle="    fn serve(source: &'static Source, item: Item) {"
    assert l.count(needle)==1
    lanes.write_text(l.replace(needle,needle+'\n        drop(std::hint::black_box(vec![0u8; 4096]));',1))
    p=run('injected-real-pool-worker','test(prepared_multithreaded_cycles_include_the_workers)');assert p.returncode!=0
    assert re.search(r'workers=Counts \{ allocations: [1-9]\d*,',p.stdout),p.stdout[-2000:]
    print('REAL-POOL-WORKER-ALLOCATION-OBSERVED',flush=True)
    lanes.write_text(l)
    disabled=t.replace('WORKER_COUNTS.add(value.since(before));','std::hint::black_box(value.since(before));');assert disabled!=t;test.write_text(disabled)
    p=run('disabled-observer','test(worker_counter_sees_an_owned_background_allocation)');assert p.returncode!=0;assert 'left: Counts { allocations: 0' in p.stdout
    print('DISABLED-WORKER-OBSERVER-REJECTED',flush=True)
    test.write_text(t)
    p=run('restored-green-scopes','not test(prepared_multithreaded_cycles_include_the_workers)');assert p.returncode==0,p.stdout[-2000:]
    print('RESTORED-SEVEN-GREEN-SCOPES-PASS',flush=True)
finally:
    lanes.write_text(l);test.write_text(t)
