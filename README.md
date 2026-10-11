# mmos-practice

이 저장소는 책의 실습 subsection을 그대로 따라가며 MMOS를 단계적으로 만들어 가기 위한 연습용 저장소입니다.

핵심 원칙:

- 작은 단위로 커밋을 쌓는다.
- 각 커밋은 책의 특정 subsection과 1:1로 대응되도록 유지한다.
- 빌드/실행은 컨테이너 내부 기준으로 맞춘다.

## 진행 방법

1. `docs/practice-roadmap.md`에서 현재 장의 목표를 확인한다.
2. 해당 단계 커밋을 checkout하여 상태를 재현한다.
3. 같은 흐름으로 로컬에서 다시 구현하고 커밋한다.

## User App 최소 빌드 (24장 준비)

실제 user ELF를 만드는 최소 파이프라인은 아래 명령으로 확인할 수 있다.

```bash
cd /home/nicesj/mmrnd/mmos/mmos-practice
make user-init PROFILE=release
make boot-disk PROFILE=release
```

- `build/user-init.elf`: `no_std + no_main + _start` 기반 최소 user app ELF
- `build/drive0.img`: `/sbin/init`로 user ELF가 주입된 ext4 부트 이미지

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

## 브랜치 정책

- 기본 브랜치: `main`
- 실습은 `main`에 선형 히스토리로 유지

## CUI 검증 도구

챕터별 검증 세트를 선택해서 한 번에 실행하려면 아래 스크립트를 사용합니다.

```bash
cd /home/nicesj/mmrnd/mmos/mmos-practice
./scripts/verify_practice_tui.py
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
