# Chalkak 사용자 가이드

[English Guide](USER_GUIDE.md)

이 문서는 일반 사용자가 Wayland + Hyprland 환경에서 Chalkak을 안정적으로 사용하는 방법을 설명합니다.

## 데모 영상

- [데모 영상 보기](https://github.com/user-attachments/assets/4e3a4de2-10b0-4131-ab49-983f3b0ceb50)

## 1. Chalkak이 잘 맞는 사용 방식

Chalkak은 다음 흐름에 최적화되어 있습니다.

1. 스크린샷 캡처(전체/영역/창).
2. 미리보기에서 결과 확인.
3. 저장/복사/삭제 또는 편집기 진입.
4. 편집 후 저장/복사.

빠른 캡처와 주석 편집을 키보드 중심으로 처리하고 싶다면 이 흐름이 가장 효율적입니다.

## 2. 실행 전 준비

Chalkak은 Wayland + Hyprland 세션을 전제로 합니다.

필수 런타임 명령:

- `hyprctl`
- `grim`
- `slurp`
- `wl-copy` (`wl-clipboard` 패키지)

환경 변수 전제:

- `HOME` 필수
- `XDG_RUNTIME_DIR` 권장

빠른 점검:

```bash
hyprctl version
grim -h
slurp -h
wl-copy --help
echo "$HOME"
echo "$XDG_RUNTIME_DIR"
```

## 3. 설치와 시작

### 소스에서 실행

```bash
git clone <repo-url> chalkak
cd chalkak
cargo run -- --launchpad
```

`--` 뒤의 인자는 Cargo가 아니라 Chalkak으로 전달됩니다.

### 시작 모드

작업 방식에 따라 다음 중 하나를 사용하세요.

- `chalkak --launchpad`: 런치패드 창부터 표시
- `chalkak --full`: 전체 화면 즉시 캡처
- `chalkak --region`: 영역 즉시 캡처
- `chalkak --window`: 창 즉시 캡처

동일 의미 별칭:

- `--capture-full`
- `--capture-region`
- `--capture-window`

캡처 플래그를 여러 개 주면 마지막 플래그가 적용됩니다.

## 4. 첫 사용 권장 순서

처음에는 아래 순서로 익히는 것을 권장합니다.

1. `chalkak --launchpad`로 시작
2. 런치패드 또는 단축키로 캡처 실행
3. 미리보기에서 결과 확인
4. 편집이 필요하면 `e`로 편집기 열기
5. `Ctrl+S` 저장 또는 `Ctrl+C` 복사

## 5. 미리보기 단계 사용법

미리보기는 최종 출력 전에 결과를 검수하는 단계입니다.

기본 단축키:

- `s`: 파일로 저장
- `c`: 클립보드 복사
- `e`: 편집기 열기
- `Delete`: 캡처 폐기
- `Esc`: 미리보기 닫기

잘못된 캡처를 저장하는 실수를 줄이려면 미리보기 단계를 반드시 거치는 것이 좋습니다.

## 6. 편집기 기본 조작

편집기 기본 단축키:

- `Ctrl+S`: 결과 이미지 저장
- `Ctrl+C`: 결과 이미지 복사
- `Ctrl+Z`: 실행 취소
- `Ctrl+Shift+Z`: 다시 실행
- `Delete` / `Backspace`: 선택 객체 삭제
- `o`: 도구 옵션 패널 토글
- `Esc`: 선택 도구로 복귀, 이미 선택 도구면 편집기 닫기

도구 단축키:

- `v`: 선택
- `h`: 패닝
- `b`: 블러
- `p`: 펜
- `a`: 화살표
- `r`: 사각형
- `c`: 크롭
- `t`: 텍스트

텍스트 편집 단축키:

- `Enter`: 줄바꿈
- `Ctrl+Enter`: 텍스트 확정
- `Ctrl+C`: 선택된 텍스트 복사
- `Esc`: 텍스트 편집 포커스 종료

## 7. 도구별 실전 팁

### 선택 (`v`)

- 객체 클릭으로 선택, 이동/리사이즈 가능
- 빈 영역 드래그로 선택 박스 생성
- `Delete`로 현재 선택 삭제

### 패닝 (`h` 또는 Space 홀드)

- 기본 패닝 홀드 키는 `Space`
- 확대 상태에서 캔버스 이동할 때 유용

### 블러 (`b`)

- 드래그로 블러 영역 지정
- 너무 작은 드래그(0 크기)는 적용되지 않음
- 현재 UI에서는 블러 강도 조절을 제공하지 않음

### 펜 (`p`)

- 드래그로 자유 곡선 그리기
- 색상/불투명도/두께 설정이 다음 스트로크에도 유지

### 화살표 (`a`)

- 시작점에서 끝점으로 드래그
- 강조 지시선에 적합
- 두께와 화살촉 크기 조절 가능

### 사각형 (`r`)

- 드래그로 사각형 생성
- 윤곽선/채우기 선택 가능
- 모서리 라운드 반경 조절 가능

### 크롭 (`c`)

- 드래그로 잘라낼 영역 프레임 지정
- 실제 크롭은 저장/복사 시 최종 렌더 단계에서 적용
- `Esc`로 크롭 취소 후 선택 도구 복귀

### 텍스트 (`t`)

- 클릭으로 텍스트 박스 생성/선택
- 기존 텍스트 더블클릭으로 편집 진입
- 현재 UI에서 노출되는 스타일 옵션은 색상과 텍스트 크기 중심임

## 8. 네비게이션/줌

기본 편집기 네비게이션:

- 패닝 홀드: `Space`
- 확대: `Ctrl++`, `Ctrl+=`, `Ctrl+KP_Add`
- 축소: `Ctrl+-`, `Ctrl+_`, `Ctrl+KP_Subtract`
- 실제 크기: `Ctrl+0`, `Ctrl+KP_0`
- 화면 맞춤: `Shift+1`

## 9. 설정 파일

설정 디렉터리:

- `$XDG_CONFIG_HOME/chalkak/`
- fallback: `$HOME/.config/chalkak/`

파일:

- `theme.json`
- `keybindings.json`

### 9.1 `theme.json`

최소 예시:

```json
{
  "mode": "system"
}
```

확장 예시:

```json
{
  "mode": "dark",
  "colors": {
    "dark": {
      "focus_ring_color": "#8cc2ff",
      "border_color": "#2e3a46",
      "panel_background": "#10151b",
      "canvas_background": "#0b0f14",
      "text_color": "#e7edf5",
      "accent_gradient": "linear-gradient(135deg, #6aa3ff, #8ee3ff)",
      "accent_text_color": "#07121f"
    }
  },
  "editor": {
    "rectangle_border_radius": 10,
    "default_tool_color": "#ff6b6b",
    "default_text_size": 18,
    "default_stroke_width": 3
  }
}
```

메모:

- `mode` 값: `system`, `light`, `dark`
- `colors.light`, `colors.dark`는 필요한 항목만 부분 지정 가능
- 누락된 값은 기본 테마 값으로 보완됨

### 9.2 `keybindings.json`

예시:

```json
{
  "editor_navigation": {
    "pan_hold_key": "space",
    "zoom_scroll_modifier": "control",
    "zoom_in_shortcuts": ["ctrl+plus", "ctrl+equal", "ctrl+kp_add"],
    "zoom_out_shortcuts": ["ctrl+minus", "ctrl+underscore", "ctrl+kp_subtract"],
    "actual_size_shortcuts": ["ctrl+0", "ctrl+kp_0"],
    "fit_shortcuts": ["shift+1"]
  }
}
```

메모:

- `zoom_scroll_modifier` 값: `none`, `control`, `shift`, `alt`, `super`
- 단축키 배열을 빈 리스트(`[]`)로 두면 오류가 발생함
- 수정자 키 별칭(`ctrl`, `control`, `cmd`, `super`)은 정규화되어 인식됨

## 10. 파일 저장 위치

임시 캡처:

- `$XDG_RUNTIME_DIR/` (예: `capture_<id>.png`)
- fallback: `/tmp/chalkak/`

최종 저장 이미지:

- `$HOME/Pictures/`

필요 시 Chalkak이 디렉터리를 자동 생성합니다.

## 11. 문제 해결

### 증상: 캡처가 시작되지 않음

가능 원인:

- `hyprctl`, `grim`, `slurp` 중 누락
- Hyprland 세션 외부에서 실행

해결:

1. 2장의 점검 명령 실행
2. `HYPRLAND_INSTANCE_SIGNATURE` 존재 확인
3. `chalkak --region`으로 다시 시도 후 유효 영역 선택

### 증상: 클립보드 복사 실패

가능 원인:

- `wl-copy` 누락 또는 실행 실패

해결:

1. `wl-copy --help` 확인
2. `wl-clipboard` 패키지 설치 여부 확인

### 증상: 저장 실패

가능 원인:

- `HOME` 미설정
- `$HOME/Pictures` 쓰기 권한 부족

해결:

1. `echo "$HOME"` 확인
2. `~/Pictures` 권한 확인

### 증상: 임시 파일이 많이 쌓임

가능 원인:

- `XDG_RUNTIME_DIR` 미설정으로 `/tmp/chalkak/` fallback 사용, 또는 런타임 디렉터리의 임시 파일 누적

해결:

1. 로그인 환경에 `XDG_RUNTIME_DIR` 설정
2. `$XDG_RUNTIME_DIR` (fallback 사용 시 `/tmp/chalkak`)의 오래된 `capture_*.png` 파일 정리

## 12. 작업 목적별 추천 흐름

### 빠른 1회성 캡처

1. `chalkak --region` 실행
2. 영역 선택
3. 미리보기에서 `c`로 즉시 복사

### 문서용 주석 캡처

1. `chalkak --window` 실행
2. `e`로 편집기 진입
3. `r`(사각형), `a`(화살표), `t`(텍스트) 활용
4. `Ctrl+S` 저장

### 민감정보 가림 후 공유

1. `chalkak --full` 실행
2. 편집기 열기
3. `b`로 민감 영역 블러 처리
4. `Ctrl+C` 복사

## 13. 빠른 명령어 요약

```bash
# 런치패드부터 시작
chalkak --launchpad

# 즉시 캡처
chalkak --full
chalkak --region
chalkak --window
```

일상 사용에서는 `--launchpad`로 익숙해진 뒤, 속도가 중요할 때 `--region`/`--window`를 사용하는 방식이 가장 실용적입니다.
