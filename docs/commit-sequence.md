# Commit Sequence for Book Practice

아래 순서대로 checkout 하며 실습하면, 책의 subsection 흐름과 거의 1:1로 따라갈 수 있습니다.

1. `a6c5a3e` `docs: initialize practice repository roadmap`
   - 실습 방식/로드맵 확인
2. `bb8a1ee` `chore(container): add podman workflow and usable run targets`
   - 컨테이너 중심 워크플로 + `run-nographic`/`run-monitor` 준비
3. `3aac907` `chore(boot): add submodule bootstrap and disk setup scripts`
   - OpenSBI/U-Boot 기본 자산과 초기 부트 준비
4. `d19e9e7` `feat(kernel): add minimal riscv boot entry with selectable sbi/mmio console`
   - 최소 커널 엔트리 + 선택형 콘솔 출력
5. `ede1e09` `test(core): add test entry layout and basic alignment tests`
   - 테스트 엔트리와 기초 단위 테스트
6. `11981bb` `feat(memory): add kheap and page frame allocator practice modules`
   - 메모리 관리 모듈 확장
7. `a2ef2c8` `feat(boot): add u-boot boot flow for chapter 7`
   - `uImage`, ext4 부트 디스크, `run-u-boot` 경로 도입
   - 부트 디스크 파티션, U-Boot 빌드 의존성, 펌웨어 clone 스크립트까지 포함
8. `ae85946` `feat(trap): add trapframe and stvec/interrupt setup scaffold`
   - Trap/IRQ 기초 구조 도입
9. `840b365` `feat(ch10): add trap dispatch and syscall scaffold`
   - trap 디스패치와 syscall(write/exit) 골격
10. `ca0f226` `feat(ch11): add sv39 page-table primitives`
    - Sv39 페이지 테이블 기본 자료구조
11. `89ca87f` `feat(ch12): add user-init context and demo exit path`
    - user init 컨텍스트 준비와 exit 경로 확인
12. `84e0986` `feat(exec): materialize init elf into user image and trap context`
    - ELF load plan을 실제 user image(복사 + BSS zero-fill)로 연결
    - TrapFrame entry/sp/sstatus를 exec 컨텍스트로 구성
13. `bd49d9f` `feat(vm): build sv39 user vm plan for init exec image`
    - init ELF/stack이 차지하는 영역을 Sv39 root table(vpn2, 1GiB leaf)로 구성
    - `satp` 값까지 함께 계산해 U-mode 진입 직전 상태를 검증
14. `6ad1b57` `feat(mm): add vm areas, page fault path, and frame manager`
    - 사용자 VM area(`VmSpace`)와 접근권한 기반 fault 해석 추가
    - page fault 경로에서 frame alloc/reuse(`PageFrameManager`) 계획 생성
15. `3dc7ec5` `feat(ch19): connect page fault results to trap outcomes`
    - page fault 결과(`Mapped/InvalidAccess/OutOfMemory`)를 trap outcome(`Resume/Exit`)으로 연결
    - user fault 처리 정책을 trap 계층에서 일관되게 소비할 수 있는 경로 고정
16. `9edf306` `feat(ch20): add timer-driven scheduler and context switch scaffolds`
    - timer tick 기반 최소 scheduler(`Scheduler`)와 run queue 순환 규칙 추가
    - context switch 저장/복원 스캐폴드(`context_switch.rs`)와 단위 테스트 추가
17. `17bd0eb` `feat(ch21): add pcb/tcb process model foundations`
    - 프로세스 생명주기용 PCB와 실행 단위용 TCB 구조 분리
    - 부모-자식 링크, zombie/reap 규칙, process/task table 기본 연산 추가
18. `1618b83` `feat(ch22): stabilize runtime demo path and process lifecycle model`
    - QEMU 부팅 로그에 `[sched] tick/switch` 시그니처를 추가해 스케줄러 경로를 시각화
    - QEMU 부팅 로그에 `[proc] fork/exec/wait` 시그니처를 추가해 프로세스 상태 전이를 시각화
    - `src/process_runtime.rs` + 단위 테스트로 fork/exec/wait 라이프사이클 모델을 별도로 고정

19. `16f0be5` `feat(ch24): add minimal user-init ELF build and boot-disk injection`
    - user 공용 링크 스크립트(`user/common/mmos_user_app.ld`)를 기준으로 `user-init.elf` 빌드 경로를 고정
    - `make boot-disk`에서 `/sbin/init`로 주입되는 실제 user ELF 아티팩트 체인을 고정
20. `3655e98` `feat(ch25): add initial user stack ABI`
    - `argc/argv/envp` 초기 user stack 레이아웃 빌더를 추가
    - verifier와 단위 테스트를 통해 stack ABI 규칙을 회귀 없이 검증
21. `162e889` `feat(ch26): add user-init launch handoff plan scaffold`
    - exec image, user vm plan, argc/argv/envp stack를 하나의 launch plan으로 결합
    - runtime 로그에 ch26 handoff 시그니처를 추가해 첫 user 진입 직전 상태를 확인
22. `95fcfb8` `feat(ch27): integrate runtime /sbin/init load path`
    - test 전용 ext4/ELF 빌더를 공용 모듈로 승격해 런타임에서도 재사용
    - `main` 경로에서 `Vfs -> load_init_exec_image("/sbin/init") -> launch handoff`를 연결
    - QEMU 로그에 `/sbin/init` 로딩 시그니처를 추가해 통합 경로를 눈으로 검증
23. `4837357` `feat(ch27): load init from virtio boot disk`
    - QEMU `virtio-blk` 장치에서 `build/drive0.img`를 직접 읽는 최소 polling 드라이버 추가
    - MBR 첫 파티션을 `PartitionBlockDevice`로 감싸 ext4/VFS가 실제 `/sbin/init`을 읽도록 런타임 경로 변경
    - `demo_user_init.rs`는 런타임 주 경로가 아니라 단위 테스트 fixture로 유지
24. `cd96cdc` `feat(ch28): execute first user-init via sret`
    - trap entry/return 경로를 연결해 U-mode로 실제 진입
    - user `write/exit` syscall이 커널 trap 경로를 왕복하며 로그를 남기는지 확인
25. `29c58d3` `feat(ch31): add character device abstraction`
    - `DeviceKind::Char`를 실제 `CharacterDevice` trait 경계와 연결
    - `CharError`와 mock 문자 장치 테스트로 read/write 처리 규칙을 고정
26. `f28af35` `feat(ch32): route serial output through character device`
    - 기존 UART 출력 경로를 `SerialDevice`와 `console::write()` 경유로 재배치
    - user `write(1, ...)` syscall이 콘솔 문자 장치 경계를 통해 출력되도록 연결

27. `316b2eb` `feat(ch33): add process fd table for stdio writes`
    - `OpenFile`과 `FileDescriptorTable`을 추가해 fd가 프로세스별 open file 슬롯 인덱스임을 코드로 고정
    - `stdin/stdout/stderr`를 console 문자 장치에 preopen하고 `write(1, ...)`이 fd table을 거치도록 변경

28. `6fcbf7a` `feat(ch34): add devfs namespace device nodes`
    - `/`, `/dev`, `/dev/console`을 해석하는 최소 devfs namespace를 추가
    - character/block device node를 표현하고 console device node를 `OpenFile`로 연결

29. `d816c1e` `feat(ch35): add open path fd allocation`
    - `/dev/console` path lookup 결과를 `OpenFile`로 만들고 fd table의 빈 슬롯에 등록
    - `SYS_OPENAT` decode와 `handle_open_path()` helper를 추가해 다음 syscall 통합 단계의 발판 고정

30. `8102cf2` `feat(ch36): keep fd table in process io context`
    - 현재 프로세스의 입출력 상태를 나타내는 `ProcessIoContext`를 추가
    - `open("/dev/console")`으로 얻은 fd가 같은 process fd table 안에 남아 이후 `write(fd, ...)`에서 재사용됨을 검증

31. `cac2997` `feat(ch37): add user copyin helpers for syscall buffers`
    - user address range를 검증하고 kernel scratch buffer로 복사하는 `UserSlice`/copyin helper 추가
    - `openat()` path와 `write()` buffer가 copyin 경계를 통과한 뒤 process fd context로 전달되는 흐름 검증

32. `dc4ae0e` `feat(ch38): check copyin against user vm areas`
    - `VmSpace::allows_range()`와 `CheckedUserSlice`를 추가해 copyin 전에 user VM area 권한을 확인
    - read 권한이 없는 영역과 영역 밖으로 넘어가는 copyin을 거부하도록 syscall helper 테스트 보강
33. `9e538dd` `feat(ch39): route checked write syscall through fd table`
    - `write(fd, user_buf, len)` 형태의 syscall helper를 추가해 checked copyin 결과를 현재 process fd table의 open file write로 전달
    - bad fd, 권한 오류, user pointer 오류를 syscall errno 형태로 구분해 반환하도록 테스트 보강

34. `b70e151` `feat(ch40): add current process io context`
    - 현재 실행 중인 process의 pid와 `ProcessIoContext`를 함께 보관하는 `CurrentProcessIoContext`를 추가
    - fd 기반 checked write syscall이 current process wrapper에서 I/O context를 꺼내 사용하도록 연결

35. `720d9b2` `feat(ch41): resolve process io from task owner`
    - `ProcessIoTable`을 추가해 task의 `owner_pid`로 현재 process I/O context를 찾는 경로를 고정
    - fd 기반 checked write syscall이 task owner lookup으로 얻은 current process I/O context를 사용하도록 검증

36. `be0ab15` `feat(ch42): create process with owned io runtime`
    - `ProcessIoRuntime`을 추가해 process 생성과 stdio I/O context 등록을 하나의 runtime API로 묶음
    - 생성된 process의 task `owner_pid` lookup으로 fd 기반 checked write syscall이 동작함을 검증

37. `2567552` `feat(ch43): add close fd lifecycle`
    - fd table에 `close(fd)`를 추가해 open file slot을 비우고 가장 낮은 fd 재사용을 검증
    - current process close syscall helper를 추가해 닫힌 fd에 대한 write가 `-EBADF`로 실패하도록 연결

38. `ac0c769` `feat(ch44): add read fd copyout path`
    - mutable user buffer view와 checked copyout을 추가해 VM write 권한을 확인한 뒤 user memory로 복사
    - `read(fd, user_buf, len)` helper를 추가해 fd table read 결과를 copyout하고 오류를 errno로 매핑

39. `7b75884` `feat(ch45): add buffered console read path`
    - runtime 검증용 buffered console character device를 추가해 실제 byte read가 가능하도록 구성
    - `read(fd, user_buf, len)`이 console input bytes를 copyout해 user memory에 저장하는 경로를 검증

40. `9e2da10` `feat(ch46): map nonblocking read wouldblock`
    - `CharError::WouldBlock`을 syscall errno `-EAGAIN`으로 매핑해 non-blocking read 실패 정책을 고정
    - buffered console input을 모두 소비한 뒤 추가 `read()`가 `-EAGAIN`으로 실패하는 경로를 검증

41. `40094ef` `feat(ch48): add blocking read wait queue`
    - `WaitQueue`를 추가해 입력이 없는 blocking read가 현재 task를 대기열에 등록할 수 있도록 구성
    - `WouldBlock`을 만난 blocking read helper가 task 상태를 `Blocked`로 바꾸고 wakeup 시 `Runnable`로 되돌리는 경로를 검증

42. `c5a175e` `feat(ch49): wake reader on serial rx event`
    - serial RX event helper를 추가해 입력 byte가 준비되었을 때 read wait queue의 task를 깨울 수 있도록 구성
    - RX event가 blocked reader를 `Runnable` 상태로 되돌리는 경로를 런타임 probe와 테스트로 검증

43. `9d580d8` `feat(ch50): hand off blocked syscall to scheduler`
    - `TrapOutcome::Schedule`을 추가해 blocking read가 user mode로 바로 복귀하지 않아야 함을 표현
    - blocked syscall 결과는 trap frame을 완료 처리하지 않고 scheduler handoff로 이어지도록 검증

44. `bfed71c` `feat(ch51): schedule next runnable task after blocked syscall`
    - blocked syscall의 trap frame을 현재 task slot에 저장한 뒤 다음 runnable task를 선택하는 scheduler handoff를 추가
    - blocked task는 `Blocked` 상태로 남기고 다음 task가 `Running` 상태가 되는 경로를 검증

45. `2413668` `feat(ch52): report idle when no runnable task exists`
    - scheduler handoff 결과를 `ScheduleDecision`으로 명시해 task 전환과 idle 진입을 구분
    - 모든 task가 blocked 상태일 때 현재 tid를 비우고 idle decision을 반환하는 경로를 검증

46. `f32c8da` `feat(ch53): map idle decision to wait-for-interrupt action`
    - idle decision을 `IdleAction::WaitForInterrupt`로 번역하는 idle 경계 모듈을 추가
    - runnable task가 없을 때 커널이 panic이 아니라 interrupt 대기 경로로 갈 수 있음을 런타임 probe로 검증

47. `6932e91` `feat(ch54): restore next task frame after schedule decision`
    - switched schedule decision이 고른 next task의 saved trap frame을 현재 dispatch frame으로 복사하는 helper를 추가
    - blocked task frame 저장 이후 next task register snapshot으로 복귀 대상이 바뀌는 경로를 검증

48. `a386d29` `feat(ch55): dispatch schedule outcome through trap handoff`
    - `TrapOutcome::Schedule`을 scheduler handoff, next task frame 복원, idle action으로 이어 주는 dispatch helper를 추가
    - trap dispatch가 user로 즉시 복귀하지 않고 scheduler decision을 적용하는 흐름을 검증

49. `adcd155` `feat(ch56): add trap runtime scheduler state`
    - trap dispatch helper가 사용할 scheduler state를 소유하는 `TrapRuntime` 경계를 추가
    - 지역 scheduler probe가 아니라 runtime object를 통해 schedule dispatch와 frame 복원이 이어짐을 검증

50. `93d4029` `feat(ch57): add global trap runtime state`
    - interrupt guard로 보호되는 global trap runtime 접근 경계를 추가
    - 실제 trap dispatch가 참조할 수 있는 전역 runtime state를 통해 schedule dispatch가 동작함을 검증


## Reproduce Example

```bash
git checkout a2ef2c8
make image
./docker.run
# container inside
./scripts/init-submodules.sh
make mmos.bin PROFILE=release CONSOLE=sbi
make uImage PROFILE=release CONSOLE=sbi
make boot-disk PROFILE=release CONSOLE=sbi
make run-u-boot PROFILE=release CONSOLE=sbi RUN_SECONDS=12
```

## Recommended Next Commit Topics

- SATP 활성화 + per-process 주소공간 전환까지 포함한 full context switch 연결
- 사용자 포인터 검증(경계/권한 검사)과 copyin/copyout 헬퍼 계층 분리
