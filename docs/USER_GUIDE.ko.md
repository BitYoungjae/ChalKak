# ChalKak 사용자 가이드

[English Guide](USER_GUIDE.md)

이 문서는 일반 사용자가 Wayland + Hyprland 환경에서 ChalKak을 안정적으로 사용하는 방법을 설명합니다.

## 데모 영상

<https://github.com/user-attachments/assets/2d2ed794-f86e-4216-b5f1-7dcb513791d4>

## 시작 가이드 (대부분 사용자)

처음 설정이라면 아래만 먼저 진행하세요.

1. 설치 후 `which chalkak`으로 실행 파일 경로 확인
2. 9.3의 Print 키 프리셋을 그대로 복사/붙여넣기
3. 9.5의 명령으로 리로드/검증
4. `chalkak --launchpad` 실행 후, 일상 사용은 5장/6장 중심으로 사용

고급 커스터마이징은 지금은 건너뛰어도 됩니다.

- 선택: 편집기 네비게이션 덮어쓰기 (`keybindings.json`)는 14.2
- 선택: 테마 커스터마이징 (`theme.json`)은 14.1
- 선택: Print 외 프리셋은 9.4

## 1. ChalKak이 잘 맞는 사용 방식

ChalKak은 다음 흐름에 최적화되어 있습니다.

1. 스크린샷 캡처(전체/영역/창).
2. 미리보기에서 결과 확인.
3. 저장/복사/삭제 또는 편집기 진입.
4. 편집 후 저장/복사.

빠른 캡처와 주석 편집을 키보드 중심으로 처리하고 싶다면 이 흐름이 가장 효율적입니다.

실전에서는 에이전틱 코딩 워크플로우에도 잘 맞습니다. 특정 영역을 캡처하고 필요한 주석을 넣은 뒤, 클립보드 이미지 붙여넣기를 지원하는 도구(예: Codex CLI, Claude Code 등, 클라이언트 지원 여부에 따라 다름)에 바로 전달할 수 있습니다. 많은 스크린샷 도구는 저장 또는 수동 첨부 단계를 먼저 요구합니다.

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

### 빠른 셋업 체크리스트 (권장)

초기 설정을 단순하게 끝내려면 아래 순서를 권장합니다.

1. 런타임 도구와 실행 파일 경로 확인:

```bash
hyprctl version
grim -h
slurp -h
wl-copy --help
which chalkak
```

1. 선택: 편집기 네비게이션 키를 바꾸고 싶을 때만 `keybindings.json`을 만드세요.

기본값을 그대로 쓸 경우 이 단계는 건너뛰면 됩니다. (파일이 없으면 ChalKak 기본값을 사용합니다.)

```bash
mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/chalkak"
cat > "${XDG_CONFIG_HOME:-$HOME/.config}/chalkak/keybindings.json" <<'JSON'
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
JSON
```

1. Hyprland 바인딩은 전용 drop-in 파일(`~/.config/hypr/chalkak.conf`)에 모으고, 메인 설정에서 `source`를 1회만 연결:

```conf
source = ~/.config/hypr/chalkak.conf
```

1. 키 조합 문법을 직접 쓰지 않도록, 권장 Print 프리셋을 그대로 적용:

```bash
CHALKAK_BIN="$(command -v chalkak)"
mkdir -p "$HOME/.config/hypr"
cat > "$HOME/.config/hypr/chalkak.conf" <<EOF
# ChalKak screenshot bindings (recommended: Print-based)
unbind = , Print
unbind = SHIFT, Print
unbind = CTRL, Print
bindd = , Print, ChalKak region capture, exec, ${CHALKAK_BIN} --capture-region
bindd = SHIFT, Print, ChalKak window capture, exec, ${CHALKAK_BIN} --capture-window
bindd = CTRL, Print, ChalKak full capture, exec, ${CHALKAK_BIN} --capture-full
EOF
```

1. 검증 후 리로드:

```bash
if [ -f "${XDG_CONFIG_HOME:-$HOME/.config}/chalkak/keybindings.json" ]; then
  jq empty "${XDG_CONFIG_HOME:-$HOME/.config}/chalkak/keybindings.json"
fi
hyprctl reload
hyprctl binds -j | jq -r '.[] | select(.description|test("ChalKak")) | [.description,.arg] | @tsv'
```

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
- `c`: 클립보드로 복사 (`image/png` + 파일 경로/링크, 붙여넣기 결과는 앱마다 다를 수 있음). 클립보드 이미지 붙여넣기를 지원하는 코딩 에이전트로 컨텍스트를 보낼 때 유용함
- `u`: `c`와 동일
- `e`: 편집기 열기
- `Delete`: 캡처 폐기
- `Esc`: 미리보기 닫기

잘못된 캡처를 저장하는 실수를 줄이려면 미리보기 단계를 반드시 거치는 것이 좋습니다.

## 6. 편집기 기본 조작

편집기 기본 단축키:

- `Ctrl+S`: 결과 이미지 저장
- `Ctrl+C`: 클립보드로 복사 (`image/png` + 파일 경로/링크, 붙여넣기 결과는 앱마다 다를 수 있음). 클립보드 이미지 붙여넣기를 지원하는 코딩 에이전트로 컨텍스트를 보낼 때 유용함
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

## 9. Hyprland 키바인딩으로 ChalKak 연결하기 (권장 기본 설정)

설치 후 대부분 사용자는 이 섹션만 설정하면 충분합니다.

Omarchy/Hyprland에서 자주 쓰는 캡처를 즉시 실행하려면 Hyprland 바인딩에 ChalKak 명령을 직접 연결하세요.

### 9.1 실행 파일 경로 확인

먼저 현재 설치 기준 실행 경로를 확인합니다.

```bash
which chalkak
```

- AUR 설치라면 보통 `/usr/bin/chalkak`
- 과거 `cargo install`을 썼다면 `~/.cargo/bin/chalkak`일 수 있음

이 경로가 실제 바인딩에서 실행될 경로와 일치해야 합니다.

### 9.2 `source` 한 줄만 먼저 연결

메인 Hyprland 설정(보통 `~/.config/hypr/hyprland.conf`)에 아래 한 줄을 유지하세요.

```conf
source = ~/.config/hypr/chalkak.conf
```

이미 `bindings.conf` 같은 파일을 source 중이라면 그 파일에 같은 `source` 줄을 넣어도 됩니다.

### 9.3 빠른 시작: 권장 프리셋 그대로 사용

키 조합 문법 자체가 부담스럽다면, `~/.config/hypr/chalkak.conf`에 아래를 그대로 붙여 넣으세요.

```conf
# ChalKak screenshot bindings (recommended: Print-based)
unbind = , Print
unbind = SHIFT, Print
unbind = CTRL, Print
bindd = , Print, ChalKak region capture, exec, /usr/bin/chalkak --capture-region
bindd = SHIFT, Print, ChalKak window capture, exec, /usr/bin/chalkak --capture-window
bindd = CTRL, Print, ChalKak full capture, exec, /usr/bin/chalkak --capture-full
```

메모:

- 기존 바인딩과 충돌하면 `unbind`가 먼저 실행되어 덮어쓸 수 있습니다.
- 본인 환경의 실제 경로에 맞게 `/usr/bin/chalkak` 부분을 바꿔야 합니다.

이렇게 하면 ChalKak 바인딩 수정이 항상 한 파일에서 끝나고, 메인 설정 파일을 반복 편집할 필요가 없습니다.

### 9.4 선택: 다른 프리셋 바로 쓰기

Print 키 대신 다른 조합이 필요하면 아래 블록 중 하나를 그대로 사용하세요.

문자 기억형 (`Alt+Shift+R/W/F`):

```conf
unbind = ALT SHIFT, R
unbind = ALT SHIFT, W
unbind = ALT SHIFT, F
bindd = ALT SHIFT, R, ChalKak region capture, exec, /usr/bin/chalkak --capture-region
bindd = ALT SHIFT, W, ChalKak window capture, exec, /usr/bin/chalkak --capture-window
bindd = ALT SHIFT, F, ChalKak full capture, exec, /usr/bin/chalkak --capture-full
```

숫자열 (`Alt+Shift+2/3/4`):

```conf
unbind = ALT SHIFT, 2
unbind = ALT SHIFT, 3
unbind = ALT SHIFT, 4
bindd = ALT SHIFT, 2, ChalKak region capture, exec, /usr/bin/chalkak --capture-region
bindd = ALT SHIFT, 3, ChalKak window capture, exec, /usr/bin/chalkak --capture-window
bindd = ALT SHIFT, 4, ChalKak full capture, exec, /usr/bin/chalkak --capture-full
```

최소 설정(영역 캡처 1개만):

```conf
unbind = , Print
bindd = , Print, ChalKak region capture, exec, /usr/bin/chalkak --capture-region
```

### 9.5 설정 반영 및 점검

```bash
hyprctl reload
hyprctl binds -j | jq -r '.[] | select(.description|test("ChalKak")) | [.description,.arg] | @tsv'
```

출력에 `ChalKak ... capture` 항목과 실행 경로가 보이면 반영된 상태입니다.

### 9.6 Omarchy 사용자 참고

Omarchy 설정은 `hyprland.conf`에서 여러 `source = ...` 파일을 로드합니다. `source = ~/.config/hypr/chalkak.conf`가 실제로 로드되는지 확인하세요.

- Dotfiles를 심볼릭 링크로 관리 중이라면 실제 편집 대상이 링크 원본 경로일 수 있습니다.
- `cargo` 설치에서 AUR 설치로 옮긴 뒤 단축키가 안 먹는 경우, 바인딩 경로가 `~/.cargo/bin/chalkak`로 남아있는지 먼저 확인하세요.

## 10. 파일 저장 위치

임시 캡처:

- `$XDG_RUNTIME_DIR/` (예: `capture_<id>.png`)
- fallback: `/tmp/chalkak/`

최종 저장 이미지:

- `$HOME/Pictures/`

필요 시 ChalKak이 디렉터리를 자동 생성합니다.

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

## 12. 작업 목적별 추천 흐름

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

## 14. 고급 설정 (선택)

대부분 사용자에게는 이 섹션이 필수가 아닙니다.

기본값을 넘어 테마/편집기 네비게이션을 직접 커스터마이징할 때만 사용하세요.

설정 디렉터리:

- `$XDG_CONFIG_HOME/chalkak/`
- fallback: `$HOME/.config/chalkak/`

파일:

- `theme.json`
- `keybindings.json`

### 14.1 `theme.json`

`theme.json`은 앱 테마 모드, UI 색상, 편집기 기본값을 설정합니다.

최소 예시:

```json
{
  "mode": "system"
}
```

권장 구조 예시 (`common` 공통값 + 모드별 덮어쓰기):

```json
{
  "mode": "system",
  "colors": {
    "common": {
      "focus_ring_color": "#8cc2ff",
      "border_color": "#2e3a46",
      "text_color": "#e7edf5"
    },
    "dark": {
      "panel_background": "#10151b",
      "canvas_background": "#0b0f14",
      "accent_gradient": "linear-gradient(135deg, #6aa3ff, #8ee3ff)",
      "accent_text_color": "#07121f"
    },
    "light": {
      "panel_background": "#f7fafc",
      "canvas_background": "#ffffff",
      "accent_gradient": "linear-gradient(135deg, #3b82f6, #67e8f9)",
      "accent_text_color": "#0f172a"
    }
  },
  "editor": {
    "common": {
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
    "dark": {
      "default_tool_color": "#f4f4f5",
      "selection_drag_fill_color": "#2B63FF33"
    },
    "light": {
      "default_tool_color": "#18181b",
      "selection_drag_fill_color": "#2B63FF14"
    }
  }
}
```

메모:

- `mode` 값: `system`, `light`, `dark`
- `system`은 런타임 데스크톱/GTK 테마 설정을 따릅니다. 런타임에서 감지할 수 없으면 다크 모드로 폴백합니다.
- `colors`와 `editor` 모두 같은 패턴으로 설정할 수 있습니다.
- `colors.common` + `colors.dark/light`
- `editor.common` + `editor.dark/light`
- 각 객체는 부분 지정이 가능하며, 비어 있는 값은 기본 테마 값으로 채워집니다.
- 병합 순서는 `내장 기본값 -> common -> 현재 모드(dark/light)`입니다.
- `colors`에서 사용할 수 있는 키:
- `focus_ring_color`, `focus_ring_glow`, `border_color`, `panel_background`, `canvas_background`, `text_color`, `accent_gradient`, `accent_text_color`
- `editor`에서 사용할 수 있는 키 (`common`, `dark`, `light` 어디서든 동일):
- `rectangle_border_radius`, `selection_drag_fill_color`, `selection_drag_stroke_color`, `selection_outline_color`, `selection_handle_color`, `default_tool_color`, `default_text_size`, `default_stroke_width`, `tool_color_palette`, `stroke_width_presets`, `text_size_presets`
- `default_tool_color`는 `RRGGBB` 또는 `#RRGGBB` 형식을 허용합니다.
- `tool_color_palette`는 엄격한 `#RRGGBB` 목록만 허용합니다 (`#` 없는 `RRGGBB`는 무시).
- 선택 색상 필드는 `#RRGGBB` 또는 `#RRGGBBAA` 형식만 허용합니다.
- `stroke_width_presets` 허용 범위: `1..=64`
- `text_size_presets` 허용 범위: `8..=160`
- 각 preset 목록은 최대 6개의 고유 값만 반영됩니다.
- 잘못된 값/중복 값은 로그 경고와 함께 무시됩니다.

레거시 호환:

- 이전 스키마도 계속 지원됩니다.
- 공통값: flat `editor`
- 모드별 값: `editor_modes.dark/light`
- 새 스키마와 레거시를 함께 쓰면 우선순위는 다음과 같습니다.
- `editor`(flat) -> `editor.common` -> `editor_modes.<mode>` -> `editor.<mode>`

### 14.2 `keybindings.json`

`keybindings.json`은 편집기 네비게이션 기본값을 덮어쓰고 싶을 때만 사용하세요.

이 파일이 없으면 ChalKak 내장 기본값이 적용됩니다.

안전한 시작 템플릿:

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
- 단축키 배열은 비워두지 않아야 함
- 수정 후 JSON 유효성 확인:

```bash
jq empty "${XDG_CONFIG_HOME:-$HOME/.config}/chalkak/keybindings.json"
```

- `keybindings.json` 파싱이 실패하면 경고 로그를 남기고 기본값으로 폴백함
