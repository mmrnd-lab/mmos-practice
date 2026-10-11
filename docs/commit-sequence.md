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
18. `686599f` `feat(ch22): add runtime scheduler/process lifecycle demo logs`
    - QEMU 부팅 로그에 `[sched] tick/switch` 시그니처를 추가해 스케줄러 경로를 시각화
    - QEMU 부팅 로그에 `[proc] fork/exec/wait` 시그니처를 추가해 프로세스 상태 전이를 시각화

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

- page fault 결과를 실제 page table install + `sfence.vma` 경로에 결합
- 실제 `sret` 진입 루틴 추가(TrapFrame + satp + stvec 연동)
- 사용자 버퍼 검증을 포함한 `write` syscall 확장
