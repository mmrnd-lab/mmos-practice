# Practice Roadmap

## Chapter-aligned Flow

1. 환경 준비
   - 컨테이너/툴체인 뼈대 작성
   - OpenSBI, U-Boot 서브모듈 준비
2. 최소 부팅
   - Rust freestanding 커널 엔트리
   - UART/SBI 출력 확인
3. 테스트 구조
   - 테스트 엔트리 정리
   - 기초 단위 테스트 작성
4. 메모리 확장
   - kheap / page frame allocator 추가
5. Trap/IRQ 기초
   - trap entry / stvec / TrapFrame 도입

## 학습 규칙

- 각 단계는 되돌릴 수 있게 작은 커밋으로 나눈다.
- 커밋 메시지는 `type(scope): summary` 형태를 사용한다.
- 재현 가능한 명령을 문서에 함께 남긴다.
