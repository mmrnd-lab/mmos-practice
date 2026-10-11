# Commit Sequence for Book Practice

아래 순서대로 checkout 하며 실습하면, 책의 subsection 흐름과 거의 1:1로 따라갈 수 있습니다.

1. `a6c5a3e` `docs: initialize practice repository roadmap`
   - 실습 방식/로드맵 확인
2. `bb8a1ee` `chore(container): add podman workflow and usable run targets`
   - 컨테이너 중심 워크플로 + `run-nographic`/`run-monitor` 준비
3. `3aac907` `chore(boot): add submodule bootstrap and disk setup scripts`
   - OpenSBI/U-Boot 기본 자산과 초기 부트 준비
4. `41b8d41` `feat(kernel): add minimal riscv boot entry with selectable sbi/mmio console`
   - 최소 커널 엔트리 + 선택형 콘솔 출력
5. `27dfe5b` `test(core): add test entry layout and basic alignment tests`
   - 테스트 엔트리와 기초 단위 테스트
6. `1ff4bbe` `feat(memory): add kheap and page frame allocator practice modules`
   - 메모리 관리 모듈 확장
7. `960cd95` `feat(boot): add u-boot boot flow for chapter 7`
   - `uImage`, ext4 부트 디스크, `run-u-boot` 경로 도입
   - 부트 디스크 파티션, U-Boot 빌드 의존성, 펌웨어 clone 스크립트까지 포함
8. `47757de` `feat(trap): add trapframe and stvec/interrupt setup scaffold`
   - Trap/IRQ 기초 구조 도입
9. `352ac4e` `feat(ch10): add trap dispatch and syscall scaffold`
   - trap 디스패치와 syscall(write/exit) 골격
10. `636b73d` `feat(ch11): add sv39 page-table primitives`
    - Sv39 페이지 테이블 기본 자료구조
11. `075d672` `feat(ch12): add user-init context and demo exit path`
    - user init 컨텍스트 준비와 exit 경로 확인

## Reproduce Example

```bash
git checkout 960cd95
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

- 실제 `sret` 진입 루틴 추가
- 사용자 주소공간 코드/스택 매핑 강화
- 사용자 버퍼 검증을 포함한 `write` syscall 확장
