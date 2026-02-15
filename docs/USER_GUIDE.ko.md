# ChalKak 사용자 가이드

[English Guide](USER_GUIDE.md)

이 문서는 일반 사용자가 Wayland + Hyprland 환경에서 ChalKak을 안정적으로 사용하는 방법을 설명합니다.

## 데모 영상

<https://github.com/user-attachments/assets/4e3a4de2-10b0-4131-ab49-983f3b0ceb50>

## 1. ChalKak이 잘 맞는 사용 방식

ChalKak은 다음 흐름에 최적화되어 있습니다.

1. 스크린샷 캡처(전체/영역/창).
2. 미리보기에서 결과 확인.
3. 저장/복사/삭제 또는 편집기 진입.
4. 편집 후 저장/복사.

빠른 캡처와 주석 편집을 키보드 중심으로 처리하고 싶다면 이 흐름이 가장 효율적입니다.

## 2. 실행 전 준비

ChalKak은 Wayland + Hyprland 세션을 전제로 합니다.

캡처/클립보드 기능에서 사용하는 런타임 명령:

- `hyprctl`
- `grim`
- `slurp`
- `wl-copy` (`wl-clipboard` 패키지, 이미지 바이트 복사 경로에서 사용)

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

`--` 뒤의 인자는 Cargo가 아니라 ChalKak으로 전달됩니다.

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
5. 미리보기에서는 `s`로 저장하거나, 편집기에서 `Ctrl+S` / `Ctrl+C` 사용

## 5. 미리보기 단계 사용법

미리보기는 최종 출력 전에 결과를 검수하는 단계입니다.

기본 단축키:

- `s`: 파일로 저장
- `c`: 클립보드로 복사 (`image/png` + 파일 경로/링크, 붙여넣기 결과는 앱마다 다를 수 있음)
- `u`: `c`와 동일
- `e`: 편집기 열기
- `Delete`: 캡처 폐기
- `Esc`: 미리보기 닫기

잘못된 캡처를 저장하는 실수를 줄이려면 미리보기 단계를 반드시 거치는 것이 좋습니다.

## 6. 편집기 기본 조작

편집기 기본 단축키:

- `Ctrl+S`: 결과 이미지 저장
- `Ctrl+C`: 클립보드로 복사 (`image/png` + 파일 경로/링크, 붙여넣기 결과는 앱마다 다를 수 있음)
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

- `Enter` / `Shift+Enter`: 줄바꿈
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
    "selection_drag_fill_color": "#2B63FF1F",
    "selection_drag_stroke_color": "#2B63FFE0",
    "selection_outline_color": "#2B63FFE6",
    "selection_handle_color": "#2B63FFF2",
    "default_tool_color": "#ff6b6b",
    "default_text_size": 18,
    "default_stroke_width": 3,
    "tool_color_palette": ["#ff6b6b", "#ffd166", "#3a86ff", "#06d6a0"],
    "stroke_width_presets": [2, 4, 8, 12],
    "text_size_presets": [14, 18, 24, 32]
  },
  "editor_modes": {
    "dark": {
      "default_tool_color": "#f4f4f5"
    },
    "light": {
      "default_tool_color": "#18181b"
    }
  }
}
```

메모:

- `mode` 값: `system`, `light`, `dark`
- `colors.light`, `colors.dark`는 필요한 항목만 부분 지정 가능
- 누락된 값은 기본 테마 값으로 보완됨
- `editor.default_tool_color`는 `RRGGBB` 또는 `#RRGGBB` 형식을 허용
- `editor.tool_color_palette`는 옵션 칩용 HEX 컬러를 엄격한 `#RRGGBB` 형식으로 받음 (`#` 없는 `RRGGBB`는 무시됨)
- 에디터 선택 UI 색상은 `editor.selection_drag_fill_color`, `editor.selection_drag_stroke_color`, `editor.selection_outline_color`, `editor.selection_handle_color`로 조정 가능
- 선택 색상 필드는 `#RRGGBB` 또는 `#RRGGBBAA` 형식만 허용
- 선택 UI 기본값은 모드별로 다르게 적용됨: 라이트 모드는 진한 뉴트럴(그래파이트), 다크 모드는 밝은 뉴트럴(징크) 톤 (`system`은 런타임에서 해석된 모드를 따름)
- `editor`는 모든 모드에 공통으로 적용되는 기본값이고, `editor_modes.dark`/`editor_modes.light`에서 필요한 필드만 모드별로 덮어쓸 수 있음
- `editor.stroke_width_presets`, `editor.text_size_presets`는 팝업 옵션 칩 목록을 제어함
- `editor.stroke_width_presets`는 `1..=64`, `editor.text_size_presets`는 `8..=160` 범위만 허용
- 각 preset 목록은 최대 6개의 고유 값만 반영되고 초과 항목은 무시됨
- 잘못된 값/중복 값은 로그 경고와 함께 무시됨

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
- `pan_hold_key`와 단축키 키 이름은 정규화되어 `ctrl`/`control`, `cmd`/`command`/`win`(`super`) 같은 별칭이 인식됨
- 각 단축키 조합은 수정자 키 외에 메인 키를 정확히 1개 포함해야 함 (예: `ctrl+plus`)
- 단축키 배열을 빈 리스트(`[]`)로 두면 오류가 발생함
- `keybindings.json` 파싱이 실패하면 경고 로그를 남기고 기본값으로 폴백함

## 10. Hyprland 키바인딩으로 ChalKak 연결하기

Omarchy/Hyprland에서 자주 쓰는 캡처를 즉시 실행하려면 Hyprland 바인딩에 ChalKak 명령을 직접 연결하세요.

### 10.1 실행 파일 경로 확인

먼저 현재 설치 기준 실행 경로를 확인합니다.

```bash
which chalkak
```

- AUR 설치라면 보통 `/usr/bin/chalkak`
- 과거 `cargo install`을 썼다면 `~/.cargo/bin/chalkak`일 수 있음

이 경로가 실제 바인딩에서 실행될 경로와 일치해야 합니다.

### 10.2 `bindings.conf`에 바인딩 추가

`~/.config/hypr/bindings.conf`에 아래처럼 추가합니다.

```conf
# ChalKak screenshot bindings (Option = ALT)
unbind = ALT SHIFT, 2
unbind = ALT SHIFT, 3
unbind = ALT SHIFT, 4
bindd = ALT SHIFT, 2, ChalKak region capture, exec, /usr/bin/chalkak --capture-region
bindd = ALT SHIFT, 3, ChalKak window capture, exec, /usr/bin/chalkak --capture-window
bindd = ALT SHIFT, 4, ChalKak full capture, exec, /usr/bin/chalkak --capture-full
```

메모:

- 기존 바인딩과 충돌하면 `unbind`가 먼저 실행되어 덮어쓸 수 있습니다.
- 본인 환경의 실제 경로에 맞게 `/usr/bin/chalkak` 부분을 바꿔야 합니다.

### 10.3 설정 반영 및 점검

```bash
hyprctl reload
hyprctl binds -j | jq -r '.[] | select(.description|test("ChalKak")) | [.description,.arg] | @tsv'
```

출력에 `ChalKak ... capture` 항목과 실행 경로가 보이면 반영된 상태입니다.

### 10.4 Omarchy 사용자 참고

Omarchy 설정은 `hyprland.conf`에서 여러 `source = ...` 파일을 로드합니다. `~/.config/hypr/bindings.conf`가 로드되는지 확인하세요.

- Dotfiles를 심볼릭 링크로 관리 중이라면 실제 편집 대상이 링크 원본 경로일 수 있습니다.
- `cargo` 설치에서 AUR 설치로 옮긴 뒤 단축키가 안 먹는 경우, 바인딩 경로가 `~/.cargo/bin/chalkak`로 남아있는지 먼저 확인하세요.

## 11. 파일 저장 위치

임시 캡처:

- `$XDG_RUNTIME_DIR/` (예: `capture_<id>.png`)
- fallback: `/tmp/chalkak/`

최종 저장 이미지:

- `$HOME/Pictures/`

필요 시 ChalKak이 디렉터리를 자동 생성합니다.

## 12. 문제 해결

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

- 이미지 바이트 복사 경로에서 `wl-copy` 누락 또는 실행 실패
- 멀티 포맷 복사 경로(미리보기 `c`/`u`, 편집기 `Ctrl+C`)에서 Wayland/GTK 클립보드 디스플레이 접근 실패, 임시 파일 읽기 실패, 파일 URI 변환 실패

해결:

1. `wl-copy --help` 확인
2. `wl-clipboard` 패키지 설치 여부 확인
3. Wayland GUI 세션에서 실행 중인지 확인 후 재시도

### 증상: 저장 실패

가능 원인:

- `HOME` 미설정
- `$HOME/Pictures` 쓰기 권한 부족

해결:

1. `echo "$HOME"` 확인
2. `~/Pictures` 권한 확인

### 증상: 임시 파일이 많이 쌓임

가능 원인:

- 정상 종료/정리 경로를 타지 못함(강제 종료/크래시 등)으로 이전 임시 파일이 남음
- `XDG_RUNTIME_DIR` 미설정으로 `/tmp/chalkak/` fallback 사용

해결:

1. 로그인 환경에 `XDG_RUNTIME_DIR` 설정
2. 가능하면 미리보기/편집기를 정상 종료(ChalKak은 닫기/삭제 시 캡처 임시 파일을 정리하고, 시작 시 오래된 `capture_*.png`를 자동 정리함)
3. 그래도 남아 있으면 `$XDG_RUNTIME_DIR` (fallback 사용 시 `/tmp/chalkak`)의 오래된 `capture_*.png` 파일 정리

## 13. 작업 목적별 추천 흐름

### 빠른 1회성 캡처

1. `chalkak --region` 실행
2. 영역 선택
3. 미리보기에서 `c`로 클립보드 복사

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

## 14. 빠른 명령어 요약

```bash
# 런치패드부터 시작
chalkak --launchpad

# 즉시 캡처
chalkak --full
chalkak --region
chalkak --window
```

일상 사용에서는 `--launchpad`로 익숙해진 뒤, 속도가 중요할 때 `--region`/`--window`를 사용하는 방식이 가장 실용적입니다.
