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
12. `c5eec09` `feat(exec): materialize init elf into user image and trap context`
    - ELF load plan을 실제 user image(복사 + BSS zero-fill)로 연결
    - TrapFrame entry/sp/sstatus를 exec 컨텍스트로 구성
13. `822b685` `feat(vm): build sv39 user vm plan for init exec image`
    - init ELF/stack이 차지하는 영역을 Sv39 mapping plan(vpn2, 1GiB leaf)으로 정리
    - PMM이 제공한 root frame 물리주소에서 `satp` 값을 계산하고, 별도 root table view에 mapping을 설치해 검증
14. `e46949a` `feat(mm): add vm areas, page fault path, and frame manager`
    - 사용자 VM area(`VmSpace`)와 접근권한 기반 fault 해석 추가
    - page fault 경로에서 frame alloc/reuse(`PageFrameManager`) 계획 생성
15. `43ee4fe` `feat(ch19): connect page fault results to trap outcomes`
    - page fault 결과(`Mapped/InvalidAccess/OutOfMemory`)를 trap outcome(`Resume/Exit`)으로 연결
    - user fault 처리 정책을 trap 계층에서 일관되게 소비할 수 있는 경로 고정
16. `2e04fd6` `feat(ch20): add timer-driven scheduler and context switch scaffolds`
    - timer tick 기반 최소 scheduler(`Scheduler`)와 run queue 순환 규칙 추가
    - context switch 저장/복원 스캐폴드(`context_switch.rs`)와 단위 테스트 추가
17. `697fbd7` `feat(ch21): add pcb/tcb process model foundations`
    - 프로세스 생명주기용 PCB와 실행 단위용 TCB 구조 분리
    - 부모-자식 링크, zombie/reap 규칙, process/task table 기본 연산 추가
18. `30b90b2` `feat(ch22): stabilize runtime demo path and process lifecycle model`
    - QEMU 부팅 로그에 `[sched] tick/switch` 시그니처를 추가해 스케줄러 경로를 시각화
    - QEMU 부팅 로그에 `[proc] fork/exec/wait` 시그니처를 추가해 프로세스 상태 전이를 시각화
    - `src/process_runtime.rs` + 단위 테스트로 fork/exec/wait 라이프사이클 모델을 별도로 고정

19. `03ae40b` `feat(ch24): add minimal user-init ELF build and boot-disk injection`
    - user 공용 링크 스크립트(`user/common/mmos_user_app.ld`)를 기준으로 `user-init.elf` 빌드 경로를 고정
    - `make boot-disk`에서 `/sbin/init`로 주입되는 실제 user ELF 아티팩트 체인을 고정
20. `7656007` `feat(ch25): add initial user stack ABI`
    - `argc/argv/envp` 초기 user stack 레이아웃 빌더를 추가
    - verifier와 단위 테스트를 통해 stack ABI 규칙을 회귀 없이 검증
21. `f1d6e68` `feat(ch26): add user-init launch handoff plan scaffold`
    - exec image, user vm plan, argc/argv/envp stack를 하나의 launch plan으로 결합
    - runtime 로그에 ch26 handoff 시그니처를 추가해 첫 user 진입 직전 상태를 확인
22. `c9cf926` `feat(ch27): integrate runtime /sbin/init load path`
    - test 전용 ext4/ELF 빌더를 공용 모듈로 승격해 런타임에서도 재사용
    - `main` 경로에서 `Vfs -> load_init_exec_image("/sbin/init") -> launch handoff`를 연결
    - QEMU 로그에 `/sbin/init` 로딩 시그니처를 추가해 통합 경로를 눈으로 검증
23. `bcff4c9` `feat(ch27): load init from virtio boot disk`
    - QEMU `virtio-blk` 장치에서 `build/drive0.img`를 직접 읽는 최소 polling 드라이버 추가
    - MBR 첫 파티션을 `PartitionBlockDevice`로 감싸 ext4/VFS가 실제 `/sbin/init`을 읽도록 런타임 경로 변경
    - `demo_user_init.rs`는 런타임 주 경로가 아니라 단위 테스트 fixture로 유지
24. `877a62e` `feat(ch28): execute first user-init via sret`
    - trap entry/return 경로를 연결해 U-mode로 실제 진입
    - user `write/exit` syscall이 커널 trap 경로를 왕복하며 로그를 남기는지 확인
25. `ba8bb0b` `feat(ch31): add character device abstraction`
    - `DeviceKind::Char`를 실제 `CharacterDevice` trait 경계와 연결
    - `CharError`와 mock 문자 장치 테스트로 read/write 처리 규칙을 고정
26. `34ef23b` `feat(ch32): route serial output through character device`
    - 기존 UART 출력 경로를 `SerialDevice`와 `console::write()` 경유로 재배치
    - user `write(1, ...)` syscall이 콘솔 문자 장치 경계를 통해 출력되도록 연결

27. `712bc4c` `feat(ch33): add process fd table for stdio writes`
    - `OpenFile`과 `FileDescriptorTable`을 추가해 fd가 프로세스별 open file 슬롯 인덱스임을 코드로 고정
    - `stdin/stdout/stderr`를 console 문자 장치에 preopen하고 `write(1, ...)`이 fd table을 거치도록 변경

28. `efe9ae5` `feat(ch34): add devfs namespace device nodes`
    - `/`, `/dev`, `/dev/console`을 해석하는 최소 devfs namespace를 추가
    - character/block device node를 표현하고 console device node를 `OpenFile`로 연결

29. `62891af` `feat(ch35): add open path fd allocation`
    - `/dev/console` path lookup 결과를 `OpenFile`로 만들고 fd table의 빈 슬롯에 등록
    - `SYS_OPENAT` decode와 `handle_open_path()` helper를 추가해 다음 syscall 통합 단계의 발판 고정

30. `480edc8` `feat(ch36): keep fd table in process io context`
    - 현재 프로세스의 입출력 상태를 나타내는 `ProcessIoContext`를 추가
    - `open("/dev/console")`으로 얻은 fd가 같은 process fd table 안에 남아 이후 `write(fd, ...)`에서 재사용됨을 검증

31. `6efaccc` `feat(ch37): add user copyin helpers for syscall buffers`
    - user address range를 검증하고 kernel scratch buffer로 복사하는 `UserSlice`/copyin helper 추가
    - `openat()` path와 `write()` buffer가 copyin 경계를 통과한 뒤 process fd context로 전달되는 흐름 검증

32. `edc3a09` `feat(ch38): check copyin against user vm areas`
    - `VmSpace::allows_range()`와 `CheckedUserSlice`를 추가해 copyin 전에 user VM area 권한을 확인
    - read 권한이 없는 영역과 영역 밖으로 넘어가는 copyin을 거부하도록 syscall helper 테스트 보강
33. `f46b61c` `feat(ch39): route checked write syscall through fd table`
    - `write(fd, user_buf, len)` 형태의 syscall helper를 추가해 checked copyin 결과를 현재 process fd table의 open file write로 전달
    - bad fd, 권한 오류, user pointer 오류를 syscall errno 형태로 구분해 반환하도록 테스트 보강

34. `715ac89` `feat(ch40): add current process io context`
    - 현재 실행 중인 process의 pid와 `ProcessIoContext`를 함께 보관하는 `CurrentProcessIoContext`를 추가
    - fd 기반 checked write syscall이 current process wrapper에서 I/O context를 꺼내 사용하도록 연결

35. `e2d853e` `feat(ch41): resolve process io from task owner`
    - `ProcessIoTable`을 추가해 task의 `owner_pid`로 현재 process I/O context를 찾는 경로를 고정
    - fd 기반 checked write syscall이 task owner lookup으로 얻은 current process I/O context를 사용하도록 검증

36. `e05b920` `feat(ch42): create process with owned io runtime`
    - `ProcessIoRuntime`을 추가해 process 생성과 stdio I/O context 등록을 하나의 runtime API로 묶음
    - 생성된 process의 task `owner_pid` lookup으로 fd 기반 checked write syscall이 동작함을 검증

37. `c54dc2e` `feat(ch43): add close fd lifecycle`
    - fd table에 `close(fd)`를 추가해 open file slot을 비우고 가장 낮은 fd 재사용을 검증
    - current process close syscall helper를 추가해 닫힌 fd에 대한 write가 `-EBADF`로 실패하도록 연결

38. `4864788` `feat(ch44): add read fd copyout path`
    - mutable user buffer view와 checked copyout을 추가해 VM write 권한을 확인한 뒤 user memory로 복사
    - `read(fd, user_buf, len)` helper를 추가해 fd table read 결과를 copyout하고 오류를 errno로 매핑

39. `fcdd437` `feat(ch45): add buffered console read path`
    - runtime 검증용 buffered console character device를 추가해 실제 byte read가 가능하도록 구성
    - `read(fd, user_buf, len)`이 console input bytes를 copyout해 user memory에 저장하는 경로를 검증

40. `0e6ae7f` `feat(ch46): map nonblocking read wouldblock`
    - `CharError::WouldBlock`을 syscall errno `-EAGAIN`으로 매핑해 non-blocking read 실패 정책을 고정
    - buffered console input을 모두 소비한 뒤 추가 `read()`가 `-EAGAIN`으로 실패하는 경로를 검증

41. `28fba0c` `feat(ch48): add blocking read wait queue`
    - `WaitQueue`를 추가해 입력이 없는 blocking read가 현재 task를 대기열에 등록할 수 있도록 구성
    - `WouldBlock`을 만난 blocking read helper가 task 상태를 `Blocked`로 바꾸고 wakeup 시 `Runnable`로 되돌리는 경로를 검증

42. `d8ab2ed` `feat(ch49): wake reader on serial rx event`
    - serial RX event helper를 추가해 입력 byte가 준비되었을 때 read wait queue의 task를 깨울 수 있도록 구성
    - RX event가 blocked reader를 `Runnable` 상태로 되돌리는 경로를 런타임 probe와 테스트로 검증

43. `9e969cb` `feat(ch50): hand off blocked syscall to scheduler`
    - `TrapOutcome::Schedule`을 추가해 blocking read가 user mode로 바로 복귀하지 않아야 함을 표현
    - blocked syscall 결과는 trap frame을 완료 처리하지 않고 scheduler handoff로 이어지도록 검증

44. `366a719` `feat(ch51): schedule next runnable task after blocked syscall`
    - blocked syscall의 trap frame을 현재 task slot에 저장한 뒤 다음 runnable task를 선택하는 scheduler handoff를 추가
    - blocked task는 `Blocked` 상태로 남기고 다음 task가 `Running` 상태가 되는 경로를 검증

45. `18156fc` `feat(ch52): report idle when no runnable task exists`
    - scheduler handoff 결과를 `ScheduleDecision`으로 명시해 task 전환과 idle 진입을 구분
    - 모든 task가 blocked 상태일 때 현재 tid를 비우고 idle decision을 반환하는 경로를 검증

46. `20ecd9a` `feat(ch53): map idle decision to wait-for-interrupt action`
    - idle decision을 `IdleAction::WaitForInterrupt`로 번역하는 idle 경계 모듈을 추가
    - runnable task가 없을 때 커널이 panic이 아니라 interrupt 대기 경로로 갈 수 있음을 런타임 probe로 검증

47. `4b02833` `feat(ch54): restore next task frame after schedule decision`
    - switched schedule decision이 고른 next task의 saved trap frame을 현재 dispatch frame으로 복사하는 helper를 추가
    - blocked task frame 저장 이후 next task register snapshot으로 복귀 대상이 바뀌는 경로를 검증

48. `f2ea044` `feat(ch55): dispatch schedule outcome through trap handoff`
    - `TrapOutcome::Schedule`을 scheduler handoff, next task frame 복원, idle action으로 이어 주는 dispatch helper를 추가
    - trap dispatch가 user로 즉시 복귀하지 않고 scheduler decision을 적용하는 흐름을 검증

49. `d298fd0` `feat(ch56): add trap runtime scheduler state`
    - trap dispatch helper가 사용할 scheduler state를 소유하는 `TrapRuntime` 경계를 추가
    - 지역 scheduler probe가 아니라 runtime object를 통해 schedule dispatch와 frame 복원이 이어짐을 검증

50. `bf3eec2` `feat(ch57): add global trap runtime state`
    - interrupt guard로 보호되는 global trap runtime 접근 경계를 추가
    - 실제 trap dispatch가 참조할 수 있는 전역 runtime state를 통해 schedule dispatch가 동작함을 검증

51. `7ffd712` `feat(ch58): route trap schedule through global runtime`
    - `TrapOutcome::Schedule`을 global trap runtime dispatch로 연결하는 trap 경계 helper를 추가
    - RISC-V trap dispatch가 schedule outcome을 무시하지 않고 global runtime decision을 적용하도록 연결

52. `0b29714` `feat(ch59): install launch task into global runtime`
    - user init launch plan의 trap frame을 global trap runtime의 initial task로 등록하는 helper를 추가
    - 실제 `/sbin/init` launch handoff에서 만든 task context가 scheduler state에 들어감을 검증


53. `bd084fe` `feat(ch61): add user process creation path`
    - launch plan을 process table, task table, scheduler에 함께 등록하는 공통 생성 경로를 추가
    - boot-time init 등록과 이후 fork/exec 계열 syscall이 공유할 user process 생성 primitive를 검증


54. `17bd102` `feat(ch62): switch to second created user task`
    - 공통 user process 생성 경로로 두 개의 launch context를 process/task/scheduler에 등록
    - 첫 task가 blocked 되었을 때 scheduler decision과 live frame 복원이 두 번째 task context로 전환되는지 검증


55. `b6b4016` `feat(ch63): resolve task address space token`
    - schedule decision 이후 next task의 owner process를 따라 process table에서 `satp`를 해석하는 address-space switch 경계를 추가
    - 두 번째 task로 frame을 복원한 뒤 다음 주소공간 token까지 함께 확인하는 런타임 probe를 추가


56. `5d7eb11` `feat(ch64): apply task address space switch token`
    - address-space switch 결과를 active `satp` slot에 적용하는 경계를 추가
    - live frame 복원 이후 next task의 address-space token이 trap return 준비 상태에 반영됨을 검증


57. `f5008a8` `feat(ch65): add satp commit boundary`
    - active `satp` 적용 결과를 실제 CSR write 직전 commit plan으로 정리
    - RISC-V `csrw satp`와 `sfence.vma` 순서를 갖는 하드웨어 경계 함수를 추가


58. `44729b3` `feat(ch66): relocate user address space for kernel mapping`
    - kernel mapping 보존 조건을 설명하고 user init/linker/demo 주소를 `vpn2=1` 영역으로 재배치
    - user root에 supervisor-only/global kernel mapping을 함께 둘 수 있음을 검증


59. `e74e921` `feat(ch67): add satp commit preflight`
    - user entry/stack mapping과 supervisor-only kernel mapping이 함께 준비되었는지 commit 전 조건을 확인
    - 실제 CSR write 전에 trap stack 전환이 남아 있음을 런타임 probe로 드러냄


60. `6885ba8` `feat(ch68): prepare kernel trap stack switch`
    - trap entry가 사용할 kernel stack frame 위치를 계산하는 preflight helper를 추가
    - user sp 보존과 kernel trap frame 공간 예약 조건을 테스트와 런타임 probe로 검증


61. `bbaeca1` `feat(ch69): prepare trap entry scratch swap`
    - trap entry가 kernel frame stack pointer를 찾을 수 있도록 `sscratch` 준비 계약을 추가
    - 첫 trap entry stack swap이 kernel stack을 활성화하고 user sp를 보존해야 함을 테스트와 런타임 probe로 검증


62. `0426b15` `feat(ch70): switch trap entry to kernel stack`
    - RISC-V trap entry assembly의 첫 동작을 `csrrw sp, sscratch, sp` 기반 stack switch로 교체
    - trap return 직전에 `sscratch`를 kernel frame sp로 복구해 다음 user trap이 같은 계약으로 다시 진입할 수 있음을 검증


63. `2dba431` `feat(ch71): prepare sscratch before entering user`
    - `enter_user`가 `sret` 전에 `sscratch` CSR에 kernel trap frame 위치를 기록하도록 연결
    - user trap entry가 기대하는 scratch 값이 CPU 안에 준비되는 조건을 테스트와 런타임 probe로 검증


64. `d346ae5` `feat(ch72): add satp hardware commit gate`
    - `SatpCommitPlan`을 실제 CSR write 대상으로 넘기기 전 Sv39 mode와 변경 여부를 검증하는 gate를 추가
    - hardware commit helper가 `csrw satp`와 `sfence.vma`를 한 경계로 묶도록 준비하고 런타임 probe로 검증


65. `580124f` `feat(ch73): prepare user launch csr sequence`
    - user launch 직전 CSR 설정 순서를 `sscratch -> satp/sfence -> sepc/sp -> sret` 계약으로 묶는 sequence를 추가
    - trap stack 준비와 satp hardware commit gate가 같은 launch path에서 일관된 값을 사용하는지 테스트와 런타임 probe로 검증


66. `1a6395e` `feat(ch74): bind satp to root page table storage`
    - PMM에서 받은 4KiB aligned root frame 주소를 `satp` PPN의 기준으로 사용
    - user VM plan을 root frame의 page table view에 설치해 placeholder root PPN 없이 `satp`를 만들 수 있음을 테스트와 런타임 probe로 검증


67. `fcab164` `feat(ch75): bind user launch to root satp`
    - user launch CSR sequence가 PMM-backed root frame에서 나온 `satp`와 같은 값을 사용해야 한다는 계약을 추가
    - root frame 기반 `satp`와 launch sequence `satp`가 어긋나면 user 진입 경계로 넘기지 않도록 테스트와 런타임 probe로 검증


68. `f98b6ee` `feat(ch76): rebind process token to root satp`
    - process table이 보관하는 address-space token을 PMM-backed root frame에서 나온 `satp`로 갱신하는 경로를 추가
    - scheduler가 고른 next task의 owner process를 따라 조회한 `satp`가 root frame 기반 값으로 바뀌는지 테스트와 런타임 probe로 검증


69. `24fcd33` `feat(ch77): thread rooted satp through commit sequence`
    - scheduler decision에서 얻은 PCB token을 active `satp` 갱신, commit plan, rooted launch sequence까지 같은 값으로 전달하는 helper를 추가
    - root frame 기반 `satp`가 commit 직전 sequence 전체에서 일관되게 유지되는지 테스트와 런타임 probe로 검증


70. `75bff0e` `feat(ch78): guard kernel runtime under satp`
    - `satp` 전환 직후에도 kernel code, kernel stack, trap entry가 새 root 아래에서 supervisor mapping으로 유지되는지 확인하는 guard를 추가
    - PMM-backed root frame의 page table view에 kernel mapping을 설치하고 kernel runtime guard를 테스트와 런타임 probe로 검증


71. `8ad5df6` `feat(ch79): introduce process address space root`
    - root frame 물리주소와 `satp` token을 함께 기록하는 `ProcessAddressSpace` 구조를 추가
    - root table 객체를 구조체 안에 복사해 들고 다니는 대신 process address space별 root frame handle을 기록함을 테스트와 런타임 probe로 검증


72. `3110a1f` `feat(ch80): bind pcb to address space handle`
    - PCB에 `address_space_id` handle을 추가하고 `ProcessAddressSpaceTable`에서 실제 address-space metadata를 찾는 경로를 추가
    - scheduler의 address-space switch가 PCB에 복사된 `satp` 대신 address-space table에서 해석한 `satp`를 사용할 수 있음을 테스트와 런타임 probe로 검증


73. `5784d1e` `feat(ch81): probe satp hardware commit roundtrip`
    - address-space table에서 얻은 `satp`를 실제 CSR에 잠깐 쓰고 `sfence.vma` 뒤 bare `satp=0`으로 복구하는 probe를 추가
    - hardware commit probe 전에 새 root table에도 kernel identity mapping이 유지되는지 확인하고 테스트와 런타임 시그니처로 검증


74. `74b9ec8` `feat(ch82): plan user image on pmm frames`
    - relocated user entry와 user stack이 낮은 identity 물리주소가 아니라 PMM이 할당한 실제 frame을 사용하도록 frame-backed image plan을 추가
    - 지속적인 `satp` 전환 전에 user code와 stack의 물리 backing을 분리해야 함을 테스트와 런타임 시그니처로 검증


75. `e7ce69e` `feat(ch83): map user pages to pmm frames`
    - frame-backed user image plan을 Sv39의 root -> L1 -> L0 table mapping으로 설치하는 helper를 추가
    - user code와 stack virtual page가 1GiB identity leaf가 아니라 PMM frame을 가리키는 4KiB leaf가 되었음을 테스트와 런타임 시그니처로 검증


76. `64fcc6a` `feat(ch84): materialize user image into pmm frames`
    - frame-backed user image plan에 따라 ELF segment bytes와 초기 user stack bytes를 PMM-backed physical frame에 복사하는 helper를 추가
    - page table leaf가 가리키는 frame 안에 실제 실행 코드와 stack 내용이 채워졌음을 테스트와 런타임 시그니처로 검증


77. `3ad6605` `feat(ch85): commit user satp persistently`
    - 이전 장들에서 준비한 root page table의 `satp`를 더 이상 bare `satp=0`으로 복구하지 않는 persistent commit 경로를 추가
    - commit 뒤에도 kernel identity mapping 아래에서 supervisor 코드가 계속 실행되고 chapter marker를 출력할 수 있음을 테스트와 런타임 시그니처로 검증


78. `bc125fe` `feat(ch86): gate supervisor user memory access`
    - S-mode가 U page를 읽고 쓸 때만 `sstatus.SUM`을 여는 `user_access` gate를 추가
    - raw user pointer를 읽는 `write` syscall 경로가 user memory access gate 안에서만 user slice를 만들도록 보강


79. `88a60ea` `feat(ch87): enter user and handle first syscall`
    - persistent `satp`와 user memory access gate 이후 저장해 둔 user launch trap frame으로 `sret` 진입을 수행
    - 같은 4KiB page 안에 놓인 text/rodata PT_LOAD segment가 같은 frame과 병합된 PTE flags를 공유하도록 보강
    - user app의 첫 `write` syscall trap, user message 출력, `exit(0)`까지 QEMU runtime 시그니처로 검증

80. `49e7539` `feat(ch90): add process-wide exit runtime model`
    - current TID에서 owner PID를 찾고, owner PCB를 zombie로 전이시키는 helper 추가
    - 같은 owner PID를 가진 모든 TCB를 non-runnable(`Zombie`) 상태로 바꾸는 TaskTable helper 추가
    - QEMU runtime marker에서 `[chapter] ch90 process exit runtime`, `[proc] exit pid=1 code=0 tasks=2` 확인

81. `1fd8a1b` `feat(ch91): hand off scheduler after process exit`
    - current task의 process-wide exit 이후 같은 owner PID의 sibling task를 scheduler 후보에서 제거
    - `schedule_after_current_exited()`로 종료한 current task에서 다른 process의 runnable task로 handoff
    - QEMU runtime marker에서 `[chapter] ch91 exit schedule handoff`, `[proc] exit handoff prev=1 next=3 owner_tasks=2 scheduler_tasks=2` 확인

82. `75633dc` `feat(ch92): dispatch trap exit through process runtime`
    - `TrapOutcome::Exit`에 가까운 dispatch helper가 process-wide exit helper와 scheduler handoff를 함께 수행
    - 종료된 process의 sibling TCB를 scheduler 후보에서 제거하고, 다음 process의 trap frame을 live frame에 복원
    - QEMU runtime marker에서 `[chapter] ch92 trap exit process dispatch`, `[proc] trap exit dispatch prev=1 next=3 restored=0x40003000` 확인

83. `e6f1eca` `feat(ch93): add process-aware trap runtime exit dispatch`
    - `ProcessTrapRuntime`이 process table, task table, scheduler를 함께 소유하도록 추가
    - runtime-owned state에서 exit dispatch를 수행해 owner PCB/TCB 상태 갱신과 next frame restore를 한 경로로 확인
    - QEMU runtime marker에서 `[chapter] ch93 process trap runtime exit`, `[proc] runtime exit owner=1 next=3 restored=0x40003000` 확인

84. `44d1aa1` `feat(ch94): reap process trap runtime zombies`
    - process-aware trap runtime에 zombie process reap 경로를 추가하고 exit code를 회수
    - reaped process owner의 TCB와 scheduler slot을 제거해 다음 process만 runnable table에 남는지 확인
    - QEMU runtime marker에서 `[chapter] ch94 process trap runtime reap`, `[proc] runtime reap pid=1 code=0 tasks=2 sched=2 next=3` 확인


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
