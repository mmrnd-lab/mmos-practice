# mmos-practice

이 저장소는 책의 실습 subsection을 그대로 따라가며 MMOS를 단계적으로 만들어 가기 위한 연습용 저장소입니다.

핵심 원칙:

- 작은 단위로 커밋을 쌓는다.
- 각 커밋은 책의 특정 subsection과 1:1로 대응되도록 유지한다.
- 빌드/실행은 컨테이너 내부 기준으로 맞춘다.

## WSL2에서 Podman 환경 준비

Ubuntu 기반 WSL2에서는 저장소의 설정 스크립트로 rootless Podman과 이미지까지 준비할 수 있다.
저장소는 Windows 드라이브(`/mnt/c` 등)가 아니라 WSL의 Linux 파일시스템 아래에 두는 것을 권장한다.

```bash
cd /home/nicesj/mmrnd/mmos/mmos-practice
./scripts/setup-wsl2-podman.sh
```

스크립트의 패키지 설치 단계에서는 `sudo` 비밀번호를 WSL 터미널에 직접 입력해야 한다. 준비가 끝나면 다음과 같이 검증한다.

```bash
./docker.run
# container inside
make test
make mmos.bin PROFILE=release
make run-nographic PROFILE=release RUN_SECONDS=8
```

Virtual Platform에 전달할 Foundation 배포 입력 산출물은 컨테이너 안에서 한 번에 빌드할 수 있다.

```bash
make dist PROFILE=release
```

이 타깃은 release kernel ELF(`target/riscv64gc-unknown-none-elf/release/mmos-practice`)와
Foundation disk image(`build/foundation.img`)를 만들고 이미지 안의 `/sbin/init`, `/bin/sh`,
`/lib/libmmos_user_libc.so` 배치를 검증한다. `.mvpkg` 패키징과 서명은 Virtual Platform이
이 두 immutable input을 측정한 뒤 담당한다.

Podman이 이미 설치되어 있다면 패키지 설치를 생략할 수 있다.

```bash
./scripts/setup-wsl2-podman.sh --skip-install
```

WSL2에서는 rootless Podman 네트워크로 `slirp4netns`를 사용한다. Podman 5의 기본값인
`pasta`가 Windows나 VPN에서 전달된 많은 host route를 복제하다
`netlink: Too many routes to duplicate`로 실패하는 경우를 피하기 위한 설정이다.
이전에 rootful Docker가 만든 `target`, `build`, `user/*/target` 산출물이 있으면 설정
스크립트가 해당 생성 디렉터리만 현재 WSL 사용자 소유로 교정한다.

## 진행 방법

1. `docs/practice-roadmap.md`에서 현재 장의 목표를 확인한다.
2. 해당 단계 커밋을 checkout하여 상태를 재현한다.
3. 같은 흐름으로 로컬에서 다시 구현하고 커밋한다.

## U-Boot 부트 체인 선택 (mmos 방식 / practice 방식)

`run-u-boot`는 `BOOT_CHAIN` 변수로 두 가지 체인을 선택할 수 있다.

```bash
# practice 기본 방식: OpenSBI -> u-boot.bin -> MMOS
make run-u-boot PROFILE=release BOOT_CHAIN=smode

# mmos 유사 방식: SPL -> u-boot.itb -> MMOS
make run-u-boot PROFILE=release BOOT_CHAIN=spl
```

- `BOOT_CHAIN=smode`:
  - `qemu-riscv64_smode_defconfig` 기반 `u-boot.bin` 사용
  - QEMU `-bios default` + `-kernel build/u-boot/u-boot.bin`
- `BOOT_CHAIN=spl`:
  - OpenSBI `fw_dynamic.bin` + `qemu-riscv64_spl_defconfig` 기반 SPL/ITB 사용
  - QEMU `-bios build/u-boot/spl/u-boot-spl` + `-device loader,file=build/u-boot/u-boot.itb,addr=0x80200000`

## User App 최소 빌드 (24장 준비)

실제 user ELF를 만드는 최소 파이프라인은 아래 명령으로 확인할 수 있다.

```bash
cd /home/nicesj/mmrnd/mmos/mmos-practice
make user-init PROFILE=release
make boot-disk PROFILE=release
```

- `build/user-init.elf`: `no_std + no_main + _start` 기반 최소 user app ELF
- `build/drive0.img`: `/sbin/init`로 user ELF가 주입된 ext4 부트 이미지

## User CRT/libc 경계 (105~107장 정리)

후반부 user runtime 흐름은 CRT와 minimal libc를 분리해서 설명한다.

- `user/crt`: final user ELF의 `_start`를 제공하고 initial stack에서 `argc/argv/envp`를 읽어 libc startup entry인 `__mmos_libc_start_main()`으로 넘긴다.
- `user/libc`: startup entry에서 app `main()`을 호출하고 return code를 `exit()`로 처리한다. syscall wrapper, stdio bootstrap, startup C-string 출력, environment lookup helper도 libc가 제공한다.
- 현재 링크 모델은 `init=ET_EXEC`, `crt=rlib`, `libc=rlib`, `shared-object=no`이다.
- shared object loader 실습에서 runtime dependency로 다루는 대상은 CRT가 아니라 `/lib/libmmos_user_libc.so`이다.

## Console stdin 연결 (108장 진행)

108장은 실제 QEMU console 입력이 user program의 `read(0, ...)`까지 도달하는 경로를 연결한다.
`SerialDevice::read()`는 선택된 console backend에서 한 byte를 받고, 최초 user process가 사용하는
간단한 syscall dispatcher도 `SYS_READ`를 user buffer copyout과 함께 처리한다.

기존 `/sbin/init` 회귀 검증은 입력을 기다리지 않도록 유지한다. 별도 `user/stdin-probe` ELF를
`/sbin/init` 위치에 넣은 `build/stdin-probe.img`로 interactive 경로를 독립 검증한다.

```bash
make run-stdin-probe PROFILE=release INPUT_BYTE=K RUN_SECONDS=6
```

성공 기준은 `[stdin-probe] received=K`와 `[user-init] exit=0`이다. 다음 단계에서는 이 raw byte 입력을
line reader와 shell prompt로 확장한다.

## 최소 user shell (109장 진행)

`user/shell`은 128 byte 고정 line buffer를 사용해 console input을 한 줄로 모은다.
CR/LF, backspace, 최대 길이를 처리하고 `help`, `echo`, `exit` builtin을 제공한다.
아직 init child process 연결 전이므로 독립 `build/shell.img`에서는 shell ELF를 `/sbin/init` 위치에 넣어 실행한다.

```bash
make run-shell PROFILE=release RUN_SECONDS=8
```

scripted input으로 `help`, `echo hello shell`, `exit`를 차례로 전달해 prompt 반복과 정상 종료를 확인한다.
다음 단계에서 같은 ELF를 `/bin/sh`에 설치하고 `/sbin/init`이 child process로 실행하도록 전환한다.

## 실제 `/sbin/init` 로딩 경로 (27장 이후)

현재 런타임 통합 경로는 메모리 안에서 만든 데모 ext4 이미지를 주 경로로 사용하지 않는다.
QEMU에 연결된 `build/drive0.img`의 virtio-blk 장치를 커널이 직접 읽고,
MBR 첫 파티션을 `PartitionBlockDevice`로 보정한 뒤 ext4/VFS에서 `/sbin/init`을 로딩한다.
QEMU 실행 타깃은 `virtio-mmio.force-legacy=false`로 modern virtio-mmio transport를 고정하고,
커널 드라이버는 `VIRTIO_F_VERSION_1`을 협상한 뒤 polling virtqueue로 512바이트 섹터를 읽는다.
교육용 ext4 reader가 현재 1024바이트 블록만 다루므로 `scripts/build-boot-disk.sh`는
`mkfs.ext4 -b 1024`로 파일시스템 블록 크기를 명시한다.
ELF 파일처럼 중간 offset에 비어 있는 구간이 생기는 sparse 파일은 ext4 direct block의
0번 hole을 0으로 채워 읽는다.

```text
make user-init
make boot-disk
QEMU virtio-blk drive0
  -> VirtioBlockDevice
  -> PartitionBlockDevice(drive0p1)
  -> Vfs::mount_ext4()
  -> load_init_exec_image("/sbin/init")
```

`src/demo_user_init.rs`는 여전히 단위 테스트 fixture로 남겨 둔다.
상위 VFS/ELF 로더를 빠르게 검증하기 위한 샘플 이미지 생성기이며, 런타임 주 경로는 실제 boot disk를 사용한다.
HEAD 런타임에서는 12장 smoke test였던 `run_user_init_demo()`를 더 이상 호출하지 않는다.
부팅 중 user app 실행은 `virtio-blk -> ext4 -> /sbin/init -> ELF load -> sret` 경로 하나로 확인한다.

## 브랜치 정책

- 기본 브랜치: `main`
- 실습은 `main`에 선형 히스토리로 유지

## CUI/CLI 검증 도구

챕터별 검증 세트를 선택해서 한 번에 실행하려면 아래 스크립트를 사용합니다.

```bash
cd /home/nicesj/mmrnd/mmos/mmos-practice
./scripts/verify_practice_tui.py
```

TUI 없이 특정 챕터를 바로 확인하려면 `--cli`를 붙입니다.
각 단계는 해당 챕터 커밋으로 checkout한 뒤 컨테이너 안에서 실행되고,
책에서 기대하는 로그/출력 시그니처가 빠지면 `OVERALL FAIL`로 종료합니다.

```bash
./scripts/verify_practice_tui.py --list
./scripts/verify_practice_tui.py --cli ch28_first_user_sret
./scripts/verify_practice_tui.py --cli ch24_user_app_build --quiet
./scripts/verify_practice_tui.py --cli --all --keep-going --quiet
```

기능:

- 대상 호스트 선택: `localhost` / `` / ``
- 챕터별 검증 세트 독립 선택/실행
- 실행 명령, 중간 로그, 최종 결과를 분할 화면에서 실시간 갱신
- `R` 실행 전 preflight 수행:
  - repo sync(`git fetch origin --prune`, `git pull --ff-only origin main`)
  - 컨테이너 이미지 재빌드(`podman build -t mmos-practice:latest .`)
- 선택한 챕터 기준 `git history`/`changes(diff)`를 TUI에서 바로 조회
- 선택한 챕터의 변경 파일을 전체 소스로 열람하고, 추가/수정 라인을 배경색으로 구분
- 소스 뷰어에서 기본 syntax highlighting(언어별 키워드/문자열/주석/숫자) 제공
- 각 검증 명령은 `podman run ... mmos-practice:latest` 컨테이너 내부에서 실행
- 이미지가 없으면 자동 빌드, 펌웨어 서브모듈이 없으면 `./scripts/init-submodules.sh` 자동 실행

조작 키:

- `Up/Down`: 챕터 이동
- `Space`: 선택/해제
- `A`: 전체 선택
- `C`: 전체 해제
- `H`: 호스트 순환
- `Y`: preflight repo sync ON/OFF
- `I`: preflight image rebuild ON/OFF
- `R`: 선택한 검증 실행
- `L`: 선택한 챕터의 git history 조회
- `D`: 선택한 챕터의 changes(diff/patch) 조회
- `F`: 선택한 챕터의 변경 파일 소스 뷰어 열기
- `S`: 실행 중지 요청
- `PgUp`/`PgDn`: Live Logs 위/아래 스크롤
- `Home`/`End`: Live Logs 맨 위/최신(tail) 이동
- `/`: Live Logs 검색어 입력
- `n` / `N`: 다음/이전 검색 결과로 이동
- `Q`: 종료(실행 중이 아닐 때)

소스 뷰어 조작 키:

- `Left`/`Right`: 이전/다음 파일
- `Up`/`Down`: 한 줄 스크롤
- `PgUp`/`PgDn`: 페이지 스크롤
- `Home`/`End`: 파일 맨 위/아래
- `B`/`Esc`/`Q`: 뷰어 닫기

환경변수(선택):

- `MMOS_PRACTICE_LOCAL`: localhost 실행 시 repo 경로 override
- `MMOS_PRACTICE_REMOTE`: 원격 실행 시 repo 경로 override
