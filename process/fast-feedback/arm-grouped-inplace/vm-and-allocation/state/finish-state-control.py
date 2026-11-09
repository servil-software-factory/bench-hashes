from pathlib import Path
import subprocess,os
w=Path('/workspace/blake3-servil/tmp/wt-finish-state-new');p=w/'src/incremental.rs';source=p.read_text();out=Path('/workspace/blake3-servil/tmp/night/nov9-overnight');(out/'finish-state-source-new.rs').write_text(source)
needle='        let input = vec![37u8; len];';assert source.count(needle)==1
try:
    p.write_text(source.replace(needle,'        let mut input = vec![37u8; len];\n        if n == 512 && len == 16384 { input.push(37); }',1).replace('STATE_SOURCE=new','STATE_SOURCE=control-plus-one',1))
    env=dict(os.environ,CARGO_TARGET_DIR='/home/b3-builds/grouped-noalloc/finish-state-new',TMPDIR='/home/b3-builds/tmp-arm-last',CC='clang-19',CARGO_BUILD_JOBS='4',CARGO_NET_OFFLINE='true')
    r=subprocess.run(['cargo','nextest','run','--release','--lib','-E','test(finish_shape_state_probe)','--success-output','immediate'],cwd=w,env=env,capture_output=True,text=True,timeout=100)
    (out/'finish-state-control.log').write_text(r.stdout+r.stderr);assert r.returncode==0
    assert 'n=512 len=16384 states={(1,' in r.stdout+r.stderr
    print('ONE-EXTRA-BYTE-MOVES-THE-STATE-ORACLE')
finally:p.write_text(source)
