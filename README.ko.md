# ChalKak

<p align="center">
  <img src="./assets/banner.jpeg" alt="ChalKak banner" width="100%" />
</p>

[English](README.md) | 한국어

Wayland + Hyprland 환경에서 동작하는 스크린샷 도구로, 미리보기 중심 흐름과 가벼운 주석 편집기를 제공합니다.

## 데모 영상

<https://github.com/user-attachments/assets/2d2ed794-f86e-4216-b5f1-7dcb513791d4>

## 사용자 가이드

- [English User Guide](docs/USER_GUIDE.md)
- [한국어 사용자 가이드](docs/USER_GUIDE.ko.md)

## 이름 유래

`ChalKak`은 카메라 셔터 소리를 뜻하는 한국어 의성어 `찰칵!`에서 따왔습니다.

## 핵심 기능

- 캡처 모드: 전체 화면, 영역, 창.
- 캡처 후 즉시 미리보기 단계 제공 (저장, 이미지 복사, 파일 참조 복사, 편집, 삭제).
- 내장 편집 도구: 선택, 패닝, 블러, 펜, 화살표, 사각형, 크롭, 텍스트, OCR.
- 미리보기/편집 모두 키보드 중심 조작 가능.
- 테마 및 편집 네비게이션 키바인딩 사용자 설정 지원.
- 시작 시 오래된 임시 캡처 자동 정리.

## 실행 요구사항

런타임 의존성:

- `hyprctl` (Hyprland 제공)
- `grim`
- `slurp`
- `wl-copy` (`wl-clipboard` 패키지)
- GTK4 런타임 라이브러리

환경 가정:

- Wayland + Hyprland 세션
- `HOME` 환경 변수 설정
- `XDG_RUNTIME_DIR` 권장 (없으면 `/tmp/chalkak` 사용)

## 설치

### AUR

이 저장소에는 `chalkak`용 AUR 패키징 메타데이터(`PKGBUILD`, `.SRCINFO`)가 포함되어 있습니다.

예를 들어 아래처럼 AUR 헬퍼로 설치할 수 있습니다.

```bash
yay -S chalkak
```

OCR 텍스트 인식 기능을 사용하려면 모델 파일도 함께 설치하세요:

```bash
yay -S chalkak-ocr-models
```

게시된 AUR 패키지 버전이 현재 crate 릴리스보다 뒤처져 있다면, 아래 소스 빌드 경로를 사용하세요.

### 소스에서 빌드

```bash
git clone https://github.com/BitYoungjae/ChalKak.git chalkak
cd chalkak
cargo run
```

## 사용법

런치패드 UI 실행:

```bash
chalkak --launchpad
```

`chalkak`를 플래그 없이 실행하면 시작 직후 종료됩니다.

시작 플래그:

- `--full` 또는 `--capture-full`
- `--region` 또는 `--capture-region`
- `--window` 또는 `--capture-window`
- `--launchpad`

일반 작업 흐름:

1. 캡처 수행 (`full`, `region`, `window`).
2. 미리보기에서 결과 확인.
3. 저장/이미지 복사/파일 참조 복사/삭제 또는 편집기로 이동.
4. 편집 후 저장/이미지 복사/파일 참조 복사.

## 기본 키바인딩

미리보기:

- `s`: 저장
- `c`: 이미지 복사
- `e`: 편집기 열기
- `o`: OCR (전체 이미지에서 텍스트 추출)
- `Delete`: 캡처 삭제
- `Esc`: 미리보기 닫기

편집기:

- `Ctrl+S`: 저장
- `Ctrl+C`: 이미지 복사
- `Ctrl+Z`: 실행 취소
- `Ctrl+Shift+Z`: 다시 실행
- `Delete` / `Backspace`: 선택 항목 삭제
- `Tab`: 도구 옵션 패널 토글
- `Esc`: 선택 도구 전환 또는 (이미 선택 모드일 때) 편집기 닫기

미리보기/편집기 액션 버튼에서도 현재 이미지의 파일 참조 복사를 지원합니다.

도구 단축키:

- `v` 선택
- `h` 패닝
- `b` 블러
- `p` 펜
- `a` 화살표
- `r` 사각형
- `c` 크롭
- `t` 텍스트
- `o` OCR

텍스트 편집:

- `Enter`: 줄바꿈
- `Ctrl+Enter`: 텍스트 확정
- `Ctrl+C`: 선택 텍스트 복사
- `Esc`: 텍스트 입력 포커스 종료

기본 편집기 네비게이션:

- 패닝 홀드 키: `Space`
- 확대: `Ctrl++`, `Ctrl+=`, `Ctrl+KP_Add`
- 축소: `Ctrl+-`, `Ctrl+_`, `Ctrl+KP_Subtract`
- 실제 크기: `Ctrl+0`, `Ctrl+KP_0`
- 화면 맞춤: `Shift+1`

## 설정 파일

설정 디렉터리:

- `$XDG_CONFIG_HOME/chalkak/`
- fallback: `$HOME/.config/chalkak/`

파일:

- `theme.json`
- `keybindings.json`
- `config.json`

`theme.json` 요약:

- `mode`: `system`, `light`, `dark`
- `config.json`: 애플리케이션 설정 (예: `ocr_language`)
- `colors`: 공통값 + 모드별 덮어쓰기 지원
- `colors.common` + `colors.dark` + `colors.light`
- `editor`: 공통값 + 모드별 덮어쓰기 지원
- `editor.common` + `editor.dark` + `editor.light`
- 각 객체는 부분 지정 가능하며 누락된 값은 내장 기본값으로 보완
- 병합 순서:
- `내장 기본값 -> common -> 현재 모드`
- `system`은 런타임 데스크톱 설정을 따르며, 감지 불가 시 dark로 폴백
- 레거시 스키마도 계속 지원:
- 공통값 flat `editor` + 모드별 `editor_modes.dark/light`
- 레거시/신규 키를 함께 쓰면 우선순위:
- `editor(flat) -> editor.common -> editor_modes.<mode> -> editor.<mode>`
- editor preset 제약:
- `stroke_width_presets`: `1..=64`
- `text_size_presets`: `8..=160`
- 각 preset 리스트: 최대 6개 고유 값

전체 예시와 필드별 상세 설명은:

- `docs/USER_GUIDE.md`
- `docs/USER_GUIDE.ko.md`

임시 캡처 저장 경로:

- `$XDG_RUNTIME_DIR/`
- fallback: `/tmp/chalkak/`

최종 이미지 저장 경로:

- `$HOME/Pictures/`

## 개발

주요 명령어:

```bash
cargo check
cargo test
cargo fmt --check
cargo clippy --all-targets --all-features -D warnings
```

모듈 구성:

- `src/app`: 런타임 오케스트레이션, GTK 라이프사이클
- `src/capture`: Hyprland/grim/slurp 캡처 백엔드
- `src/preview`: 미리보기 동작
- `src/editor`: 편집기 모델/도구 동작
- `src/input`: 단축키, 네비게이션 처리
- `src/storage`: 임시/저장 수명주기와 정리
- `src/theme`, `src/ui`: 테마/스타일 토큰
- `src/state`: 앱 상태 머신
- `src/clipboard`: 클립보드(`wl-copy`) 연동
- `src/ocr`: OCR 텍스트 인식 (PaddleOCR v5 / MNN)
- `src/config`: 설정/키바인딩/테마 경로 헬퍼
- `src/error`: 애플리케이션 공통 에러/결과 타입
- `src/logging`: tracing subscriber 초기화

## AUR 패키징 메모 (유지보수자용)

`PKGBUILD`와 `.SRCINFO`는 이 저장소에 함께 관리됩니다.

새 버전 릴리스 시:

1. `PKGBUILD`의 `pkgver`를 `Cargo.toml`의 `version`과 맞춥니다.
2. `pkgver` 변경 시 `pkgrel=1`로 초기화합니다.
3. `source`를 `.../archive/refs/tags/vX.Y.Z.tar.gz`로 갱신합니다.
4. `updpkgsums`로 체크섬을 갱신합니다.
5. `makepkg --printsrcinfo > .SRCINFO`로 `.SRCINFO`를 재생성합니다.

기본 의존성:

- `depends=('gtk4' 'hyprland' 'grim' 'slurp' 'wl-clipboard')`
- `makedepends=('rust' 'cargo' 'pkgconf' 'gtk4' 'cmake')`
- `optdepends=('chalkak-ocr-models: OCR 텍스트 인식 지원')`

패키지명 목표: `chalkak`.

별도 AUR 패키지 `chalkak-ocr-models`가 OCR용 PaddleOCR v5 모델 파일을 제공합니다. 패키징 메타데이터는 `aur/chalkak-ocr-models/`에 있습니다.

## 유지보수자

- 이름: `BitYoungjae`
- 이메일: `bityoungjae@gmail.com`

## 라이선스

`chalkak`은 다음 이중 라이선스를 사용합니다.

- MIT
- Apache-2.0

SPDX 표현식: `MIT OR Apache-2.0`

의존성 라이선스 분포(대부분 MIT/Apache 계열 permissive)와 배포 편의성을 기준으로 결정했습니다.
