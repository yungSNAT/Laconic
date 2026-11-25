<p align="center">
  <picture>
    <source media="(max-width: 600px) and (prefers-color-scheme: light)" srcset=".github/assets/banner-phone-light.svg">
    <source media="(max-width: 600px)" srcset=".github/assets/banner-phone.svg">
    <source media="(prefers-color-scheme: light)" srcset=".github/assets/banner-light.svg">
    <img src=".github/assets/banner.svg" width="100%" alt="Laconic. 답부터 먼저. 사실을 지키도록 만들었습니다. 호주 영어. 슬래브 세리프 블록 글자로 된 Laconic 워드마크 옆에 볏 달린 스파르타 투구를 그린 픽셀 아트.">
  </picture>
</p>

<div align="center">

[![tests](https://github.com/yungSNAT/Laconic/actions/workflows/tests.yml/badge.svg?branch=main)](https://github.com/yungSNAT/Laconic/actions/workflows/tests.yml) [![Codecov](https://codecov.io/gh/yungSNAT/Laconic/graph/badge.svg)](https://codecov.io/gh/yungSNAT/Laconic) [![CodeQL](https://github.com/yungSNAT/Laconic/actions/workflows/codeql.yml/badge.svg?branch=main)](https://github.com/yungSNAT/Laconic/actions/workflows/codeql.yml) [![OSV-Scanner](https://github.com/yungSNAT/Laconic/actions/workflows/osv-scanner.yml/badge.svg?branch=main)](https://github.com/yungSNAT/Laconic/actions/workflows/osv-scanner.yml) [![SonarCloud](https://sonarcloud.io/api/project_badges/measure?project=EquityBot_Laconic&metric=alert_status)](https://sonarcloud.io/summary/new_code?id=EquityBot_Laconic) [![Codacy code quality](https://app.codacy.com/project/badge/Grade/623f5ab312d4497c9e8161e547c95f44)](https://app.codacy.com/gh/yungSNAT/Laconic/dashboard)<br>
[![OpenSSF Scorecard](https://raw.githubusercontent.com/ryanduguid/.github/badges/yungSNAT/Laconic.svg)](https://scorecard.dev/viewer/?uri=github.com/yungSNAT/Laconic) [![SLSA 빌드 출처](.github/assets/badges/slsa.svg)](https://github.com/yungSNAT/Laconic/attestations) [![VirusTotal](.github/assets/badges/virustotal.svg)](https://github.com/yungSNAT/Laconic/releases/latest)<br>
[![release](https://img.shields.io/github/v/release/yungSNAT/Laconic?filter=laconic-check-v*&color=66023C&labelColor=000000&style=flat)](https://github.com/yungSNAT/Laconic/releases?q=laconic-check-v&expanded=true) [![licence: Apache-2.0](.github/assets/badges/licence.svg)](LICENSE) [![fork of: i-have-adhd](.github/assets/badges/fork-of.svg)](https://github.com/ayghri/i-have-adhd)

[설치](#install) · [사용](#use) · [달라지는 점](#what-changes) · [평가](#evaluation) · [검사기](#checker) · [항상 켜기](#always-on) · [개발](#development) · [크레딧](#credits) · [라이선스](#licence)

[English](README.md) · [Español](README.es.md) · 한국어 · [简体中文](README.zh-CN.md) · [日本語](README.ja.md)

</div>

Laconic은 코딩 에이전트와 채팅 어시스턴트를 위한 AI 'humanizer'(휴머나이저)이자 'no AI slop'(AI 군더더기 없음) 스킬입니다. 모델이 답을 먼저 말하고, 확인하지 않은 것을 표시하며, 답이 완성되면 멈추게 합니다. 또한 독자에게 필요한 모든 사실, 단계, 단서를 지키라고 지시합니다. 모든 원문 문장을 Australian Government Style Manual에 따라 쓰되, 스킬에 적힌 자체 규칙을 우선하도록 지시합니다.

AI가 쓴 글을 사람이 쓴 글처럼 바꾸려면 에이전트에게 휴머나이즈해 달라고 요청하세요. 산문 스킬은 AI 글쓰기 패턴을 없애고, 모든 사실, 숫자, 단서를 지키도록 작성되어 있으며, 꾸며 낸 개성으로 부풀리지 않고 글을 더 짧게 만듭니다. 이 약속을 검증하는 [사실 보존 벤치마크](evals/README.md#fact-survival-benchmark)는 아직 결과가 나오지 않았습니다.

<table>
<tr>
<td align="center" width="33%"><h3>5점 만점에 4.92</h3>휴머나이즈 품질, 상대는 4.54와 4.50<br><sub><a href="#against-other-humanisers">중립 루브릭, 텍스트 20개, 블라인드 심사</a></sub></td>
<td align="center" width="33%"><h3>원문 길이의 65%</h3>두 경쟁 도구는 모두 81%<br><sub><a href="#against-other-humanisers">길이만 비교, 사실 검사는 같은 표에 함께 제시</a></sub></td>
<td align="center" width="33%"><h3>4.63 대 4.07</h3>단순한 'Answer concisely.'와 비교<br><sub><a href="#evaluation">자체 루브릭, 사례 45개, 차단 결함 6개 대 29개</a></sub></td>
</tr>
</table>

3개 수치 모두 2026년 10월 20일 Laconic 자체 평가 도구에서 생성 모델 하나로 얻었습니다. 한계와 재실행 명령은 [평가](#evaluation)에 있습니다.

<a id="install"></a>
## 설치

에이전트에 다음을 붙여 넣으세요.

```text
Install the laconic skill/plugin from https://github.com/yungSNAT/laconic, following the repository's AGENTS.md.
```

또는 사용하는 에이전트의 [설치 안내](INSTALL.md)를 따르세요. Claude Code의 경우:

```bash
claude plugin marketplace add yungSNAT/laconic
claude plugin install laconic@laconic
```

Skills CLI가 지원하는 모든 에이전트의 경우:

```bash
npx skills add yungSNAT/laconic --global --agent "*" --yes
```

<a id="use"></a>
## 사용

에이전트를 다시 시작한 뒤 `/laconic`을 입력하세요. 'stop laconic'이라고 말할 때까지 세션 동안 켜져 있습니다.

<a id="what-changes"></a>
## 달라지는 점

<table>
<tr>
<td width="50%">

### 이전

> Great question! Let me take a look. Your auth flow has a few moving pieces: the middleware, the token verification and the cookie handling. It's likely a caching issue, which typically resolves itself within an hour or so. Hope this helps! Let me know if you want to dig deeper.

</td>
<td width="50%">

### 이후

> The 401 comes from the missing `Authorization` header: the endpoint requires a bearer token and the request sends none.
>
> 1. Add `Authorization: Bearer <token>` at `src/api.ts:42`
> 2. Run `npm test -- api.spec.ts`
>
> Not verified: whether the token expires mid-session. The logs do not show it.

</td>
</tr>
</table>

예시는 Laconic이 쓰는 언어인 영어 그대로 둡니다.

<a id="what-it-includes"></a>
## 구성 요소

| 구성 요소 | 경로 | 적용 시점 |
|---|---|---|
| 세션 모드 | `skills/laconic/SKILL.md` | `/laconic`을 입력한 뒤 'stop laconic'까지 세션 내내 |
| 산문 스킬 | `skills/laconic-prose/SKILL.md` | 초안 작성, 편집, 산문 감사를 위해 모델이 불러오며 해당 작업에만 적용 |
| 간결 규칙 | `rules/laconic.md` | 에이전트 규칙에 추가하면 항상 로드됨(약 240단어) |

세션 규칙 전문은 [SKILL.md](skills/laconic/SKILL.md)에 있습니다.

1. 답을 먼저 말하고, 그다음 근거를 대세요.
2. 무엇을 어떻게 아는지 말하세요. 확인하지 않은 주장은 표시하세요. '가진 증거로는 판단할 수 없습니다'도 받아들일 만한 답입니다.
3. 길이는 독자가 결정하고, 하고, 이해해야 할 것을 따릅니다. 단어는 줄이되 내용은 절대 줄이지 마세요.
4. 절차에는 번호를 매기세요.
5. 현재 작업에 머무르세요.
6. 상태를 드러내 두세요.
7. 오류는 분명히 말하세요. 오류, 알려진 원인, 해결책이나 다음 확인 사항을 말하세요.
8. 근거가 있을 때만 추정하세요.
9. 긴 목록은 묶되 관련 항목을 빠뜨리지 마세요.
10. 서두, 요약, 맺음말은 쓰지 마세요.

모든 구성 요소는 호주 영어(철자, 작은따옴표, 쉼표, 숫자, 날짜, 엠 대시 금지)를 적용하며, 호주 자료로 형태가 정해지지 않을 때는 미국 영어가 아니라 영국 영어를 따릅니다. 자체 규칙: Style Manual이 'while'과 'among'을 선호하는 곳에서도 whilst와 amongst를 씁니다. 산문 스킬은 글쓰기, 편집, 감사의 전체 흐름을 더합니다. 흔한 AI 글쓰기 패턴에 대한 빠른 점검, 의미와 목소리 보존, 매체별 안내, Style Manual 세부 규칙, AGLC4 인용 메모가 포함됩니다.

<a id="evaluation"></a>
## 평가

2026년 10월 19일 `evals/`의 평가 도구로 측정했습니다. 사례 30개, 사례당 2회 시행, Claude Opus 5.5가 생성하고 블라인드로 심사했습니다. 각 구성에는 스킬, 규칙, 참고 자료가 설치되어 읽을 수 있었습니다. 비교 구성은 Laconic이 대체하는 No AI Slop 스킬과 그 간결 규칙 stop-slop이었습니다.

| 지표 | Laconic | No AI Slop 구성 | 차이(95% 구간) |
|---|---|---|---|
| 정확성(1~5) | 4.68 | 3.98 | +0.70 (+0.33 ~ +1.07) |
| 간결성(1~5) | 4.67 | 4.12 | +0.55 (+0.30 ~ +0.80) |
| 안전성(1~5) | 4.90 | 4.85 | +0.05 (−0.10 ~ +0.20) |
| 가중 점수 | 4.74 | 4.39 | +0.35 (+0.18 ~ +0.54) |
| 차단 결함 | 1 | 7 | |
| 평균 답변 길이(문자) | 1,360 | 1,663 | |

<details>
<summary>두 번째 심사자, 검사기 집계, 후속 실행</summary>

다른 모델 계열의 두 번째 블라인드 심사자(GPT-6 Pro)도 방향에는 동의했지만 격차는 더 작았습니다. 가중 점수 4.63 대 4.46(차이 +0.17, 95% 구간 +0.05 ~ +0.30), 정확성 +0.33(+0.12 ~ +0.58)입니다. 이 심사자는 Laconic의 세부 규칙을 훨씬 엄격하게 적용해 두 구성의 많은 답변에서 연속 쉼표(serial comma)와 글자로 쓴 숫자를 찾아냈습니다.

`tools/laconic-check`는 심사자 없이 그런 실수를 셉니다. 구성당 같은 답변 60개에서:

| 구성 | 지적 없는 답변 | 연속 쉼표 후보 | 글자로 쓴 숫자 | 큰따옴표 |
|---|---|---|---|---|
| No AI Slop 구성 | 30 | 32 | 17 | 67 |
| Laconic 간결 규칙 | 40 | 13 | 15 | 1 |
| Laconic 세션 모드(`/laconic`) | 51 | 12 | 3 | 0 |

2026년 10월 20일 후속 실행(사례 30개, 1회 시행)은 이후의 규칙 변경을 측정했습니다. 간결 규칙에 숫자 규칙을 추가하자 글자로 쓴 숫자는 5에서 0으로 줄었고 지적 없는 답변은 30개 중 18개에서 23개로 늘었습니다. 세션 모드는 차단 결함 없이 가중 4.71을 받았고, 간결 규칙만 쓴 경우는 4.59였습니다. 줄인 세션 스킬(1,153단어 대신 849단어)은 더 낮은 점수(4.61)와 차단 결함 3개를 받아 전체 문구를 유지했습니다.

2026년 10월 20일의 추가 확인:

- **보내기 전에 초안 검사:** `laconic-check` 자체 검사를 쓴 세션 모드는 산문과 회계 사례 18개에서 가중 4.82를 받아 자체 검사가 없을 때의 4.71보다 높았고(회계 사례 10개에서는 4.74 대 4.49), 차단 결함은 0개 대 1개였습니다.
- **실사용:** Laconic이 이전 구성을 대체한 날 작성자 본인의 Claude Code 세션에서 `laconic-check` 지적이 없는 어시스턴트 메시지는 26%에서 42%로 늘었고, 글자로 쓴 숫자는 1,000단어당 5.7에서 3.6으로 줄었습니다.
- **에이전트 비용:** 작은 저장소 수정 5건(`evals/README.md`의 에이전트 비용 벤치마크, 2회 시행)에서 모든 구성이 모든 작업을 통과했습니다. 세션 모드는 주로 항상 로드되는 프롬프트가 더 커서 작업당 간결 규칙보다 약 27% 더 들었습니다. 별도의 에이전트 안내 절은 아무것도 줄이지 못해 삭제했습니다.

2026년 10월 20일 간결 대조 실험(사례 45개, 2회 시행, Opus 5.5가 생성하고 블라인드 심사)은 설치된 구성을 비교했습니다. 산문 스킬을 포함한 간결 규칙을 상투 문구, 덧붙인 단서, 회계 연도 관련 변경 전후로 나누어 단일 지시 'Answer concisely.'와 비교했습니다.

| 지표 | Laconic | 변경 전 Laconic | 'Answer concisely.' |
|---|---|---|---|
| 가중 점수 | 4.63 | 4.63 | 4.07 |
| 정확성(1~5) | 4.52 | 4.56 | 3.41 |
| 차단 결함 | 6 | 5 | 29 |
| 통과한 정규식 검사 | 18개 중 18 | 18개 중 16 | 18개 중 16 |
| `laconic-check` 지적 없는 답변 | 90개 중 67 | 90개 중 69 | 90개 중 41 |
| 평균 답변 길이(문자) | 1,232 | 1,148 | 1,022 |

Laconic은 단순 지시보다 가중 점수가 0.56 높습니다. 변경은 심사 품질에 중립적이며, 다른 두 구성이 놓친 회계 연도 하이픈을 고칩니다. 차단 결함 중 바뀐 안내에서 나온 것은 없습니다.

</details>

한계: 생성 모델 하나에서 나온 구성당 답변 18~90개입니다. 사례는 `evals/cases.jsonl`, 루브릭은 `evals/rubric.md`에 있습니다. 이 사례들은 규칙 문구를 다듬는 데도 쓰였으므로 점수는 보류 데이터 시험이 아닙니다. 연속 쉼표가 여전히 가장 흔한 실수입니다.

<a id="against-other-humanisers"></a>
### 다른 휴머나이저와 비교

2026년 10월 20일, 설치된 3개 구성이 호주 독자를 위해 AI가 쓴 텍스트 20개(`evals/humanise-cases.jsonl`: 고객 이메일, PR 설명, 릴리스 노트, 이사회 보고서, 채용 공고 등)를 휴머나이즈했고, 각 2회 시행으로 Claude Opus 5.5가 생성하고 블라인드 심사했습니다. 각 구성은 Skill과 Read 도구와 함께 자기 스킬만 가졌고 그것을 쓰라고 지시받았습니다. Laconic의 산문 스킬(커밋 `c7e3048`), [humanizer](https://github.com/blader/humanizer)(커밋 `8c307f3`), [avoid-ai-writing](https://github.com/conorbronsdon/avoid-ai-writing)(커밋 `cb16d10`)입니다.

| 지표 | Laconic | `humanizer` | `avoid-ai-writing` |
|---|---|---|---|
| 가중 점수, 중립 루브릭(1~5) | 4.92 | 4.54 | 4.50 |
| 가중 점수, 자체 루브릭 | 4.87 | 4.50 | 4.42 |
| 정확성, 중립 루브릭 | 4.80 | 4.13 | 4.18 |
| 간결성, 중립 루브릭 | 4.95 | 4.20 | 3.95 |
| 차단 결함 | 0 | 0 | 0 |
| 통과한 정규식 사실 및 패턴 검사(아래 참고) | 40개 중 40 | 40개 중 40 | 40개 중 35 |
| 원문 대비 평균 길이, 단어 기준 | 65% | 81% | 81% |
| 텍스트당 avoid-ai-writing 탐지기 지적(원문: 2.6) | 0.03 | 0 | 0.05 |
| 텍스트당 dslop 지적, 리듬 지표 포함(원문: 1.80) | 0.50 | 0.83 | 0.63 |
| 텍스트당 `laconic-check` 0.1.0 지적(원문: 1.85) | 0.15 | 0.45 | 0.28 |
| 텍스트 40개 생성 비용 | $4.54 | $5.84 | $15.78 |

Laconic의 자체 세부 규칙(숫자, 연속 쉼표, 엠 대시)은 빼되 호주식 철자는 여전히 요구하는 중립 루브릭에서 Laconic은 가중 점수로 `humanizer`보다 0.38, `avoid-ai-writing`보다 0.43 앞섭니다. 두 시행과 함께 텍스트 20개를 다시 표집하는 대응 부트스트랩의 95% 구간은 +0.26 ~ +0.51과 +0.26 ~ +0.59입니다. 그 세부 규칙을 루브릭에서 빼도 우위는 유지됩니다. 다른 구성에 대한 심사 메모에는 자기소개서에 덧붙인 경력, 보조금 신청서에서 추론해 낸 달러 금액처럼 원문에 없던 내용이 있습니다. 이 표본에서 3개 구성 모두 이 탐지기들의 지적을 원문 수준보다 훨씬 낮췄습니다.

<details>
<summary>정규식 검사를 수정한 방법</summary>

충실한 바꿔 쓰기('in stages', 'a second person will check', 'five to seven', 'Don't install any personal')로 실패한 답변을 살펴본 뒤 정규식 검사 4개를 넓혔습니다. 수정한 같은 검사를 모든 구성에 적용했으며, 표는 수정 후 결과입니다. 변경 전에는 40개 중 Laconic이 37개, `humanizer`가 34개를 통과했습니다. `laconic-check` 행은 버전 0.1.0 결과이며, 이후 버전은 규칙을 더해 더 많은 지적을 셉니다.

</details>

한계: 생성 모델 하나에서 나온 구성당 텍스트 40개이고, 같은 모델 계열이 이 프롬프트 20개와 이 설치 구성으로 심사했습니다. 각 검사기는 작성자의 기준을 반영하며, `avoid-ai-writing` 탐지기는 그 스킬 자체의 엔진입니다. 정규식 검사 통과는 사실이 살아남았다는 부분적 증거일 뿐, 모든 단서가 살아남았다는 증명이 아닙니다. 재실행 명령은 [evals/README.md](evals/README.md)에 있습니다.

<a id="checker"></a>
## 검사기

`tools/laconic-check`는 의존성이 없는 작은 Rust CLI입니다. `lint`는 Markdown 산문에서 다음 후보를 지적합니다. 엠 대시, 미국식 철자, 접속사와 전치사로 쓴 'while'과 'among', 큰따옴표, 연속 쉼표로 보이는 것, 글자로 쓴 숫자, '5 %'나 '$ 27.99'의 공백, '21st of September'나 연대 '1980's'처럼 Style Manual 형식이 아닌 날짜와 시각, 날짜와 맞지 않는 요일, 라틴어 약어, 챗봇식 맺음말, 'Bottom line:'이나 'circle back' 같은 상투 문구, 꾸민 대조('it's not X, it's Y'; 'not just X but Y'), 동사 없는 폭로형 질문('The result? A 40% cut.'), 'since the dawn of time' 같은 시대를 초월한 첫머리, 'open-source'나 'e-mail' 같은 불필요한 하이픈입니다. 코드, URL, 인용문은 건너뜁니다. 링크 안을 포함해 폭 없는 문자, 인용 표시, ChatGPT 추적 태그 같은 채팅 도구 잔여물과, 텍스트를 숨기거나 순서를 바꿀 수 있는 보이지 않는 문자도 지적합니다. 유니코드 태그 문자의 연속(잉글랜드, 스코틀랜드, 웨일스 국기는 예외)과 양방향 제어 문자입니다. `lint`를 실패시키지 않는 조언으로는, 2개 이상의 문장에 절 대시(엠 대시, 또는 앞뒤에 공백이 있는 엔 대시, 하이픈, 이중 하이픈)가 있는 문단(삽입구 하나를 감싼 한 쌍은 한 번으로 셈), 'could potentially'와 'may possibly' 같은 겹친 얼버무림, 굵은 라벨('**Cause:** ...')로 시작하는 연속된 글머리표 3개 이상을 지적합니다. `preserve`는 편집이 빠뜨린 숫자, 크기와 단위('$47.3 million'에서 '$47.3 billion', '150 km'에서 '150 miles'), 날짜가 붙은 월, 'Thursday' 같은 요일 이름(0.6.0부터), 인라인 코드, URL, 인용, 'not', 'must', 'will', 'if', 'before', 'likely' 같은 단어, 'up to'와 'about 50%' 같은 범위 표현을 보고하고 그것 때문에 실패합니다. `--loose`를 쓰면 'often', 'some', 'can', 'will', 'because', 'likely'처럼 빈도, 양, 능력, 예측, 원인, 얼버무림을 나타내는 잃은 단어는 대신 확인 목록에 오릅니다. 과감한 바꿔 쓰기는 사실을 바꾸지 않고도 그런 단어를 흔히 빠뜨리기 때문입니다. 잃은 얼버무림은 여전히 주장을 과장할 수 있으므로 목록의 단어를 하나하나 읽으세요(0.6.0부터). 숫자는 부호, 통화, 그리고 금액이나 천 단위 쉼표가 있는 수치를 감싼 회계용 괄호를 유지하므로 '-3%'에서 '3%', '(1,200)'에서 '1,200', '$500'에서 '£500'은 변경입니다. '(62%)'처럼 괄호 안의 다른 숫자는 삽입구입니다. 통화는 숫자 앞의 기호('$', '£', '¥' 또는 '€', '₹', '₩'처럼 유니코드 Currency Symbols 블록의 기호)이거나 숫자 뒤의 'dollars', 'euros', 'yen'이므로 '$30 billion'과 '30 billion dollars'는 일치합니다. 숫자 뒤의 'Pounds'는 어느 한쪽 텍스트가 그 숫자와 함께 '£'를 쓰면 금액이고, 그렇지 않으면 무게입니다. 통화 단어, Currency Symbols 블록의 나머지, 'pounds'는 0.6.0부터입니다. 'three'와 '3', '1000'과 '1,000', '5 per cent'와 '5%'를 같은 값으로 세고, '45 to 60 minutes'나 '45-60 minutes'의 두 경계를 모두 분으로 읽으며(0.6.0부터), 'didn't'와 'did not'은 같게, 'can't'와 'won't'는 다르게 읽습니다. '5:00 p.m.'과 '5pm'은 같은 시각으로 읽지만, '5:30' 같은 콜론 숫자는 'at'이나 'until' 같은 단어 뒤, 'am'이나 'pm' 앞, 또는 시각 뒤에 오는 'to'나 'and' 뒤에서만 시각이고 그 밖에는 비율처럼 숫자 2개입니다(0.6.0부터). 굵게 쓴 것을 포함해 두 자리까지의 순서 목록 표시는 무시합니다. 지어낸 수치처럼 편집이 더한 것도 조언으로 나열하며, 아무것도 더하면 안 되는 재작성에는 `--strict`가 그것 때문에도 실패합니다. 편집이 잃거나 더한 대문자 단어 2~3개짜리 이름은 확인 목록에 오르고 실패를 일으키지 않습니다. 가능한 반전도 마찬가지입니다. 원문 문장이 부정한 것을 편집 문장이 부정 없이 진술하는 경우로, 'is not cast'에서 'is cast'가 그 예입니다. 꾸민 대조의 부정된 절반을 빼면 부정어도 함께 사라지므로 목록에 오르지 않습니다. AI Humanizer Benchmark의 2026-10과 2026-11 주기 상용 재작성 693개에서 4건을 나열했고 모두 오경보였습니다. 2건은 다른 말로 의미를 유지했고('So stop guessing'; 'Otherwise ...'), 2건은 다른 주장을 하는 문장과 짝지어졌습니다(0.6.0부터). 마지막 줄은 공백으로 구분한 단어 수로 두 길이와, 원문 대비 편집본의 비율을 보여 줍니다. 의미를 읽지는 않으므로 깨끗한 결과는 이런 손실만 배제합니다. 지적은 살펴볼 이유일 뿐 결함의 증거가 아닙니다.

```bash
cargo run --manifest-path tools/laconic-check/Cargo.toml -- lint README.md
cargo run --manifest-path tools/laconic-check/Cargo.toml -- preserve original.md edited.md
```

`lint`와 `preserve`는 WebAssembly로 컴파일되어 브라우저의 [yungsnat.github.io/Laconic](https://yungsnat.github.io/Laconic/)에서도 동작하며, `main`에서 빌드됩니다. 페이지는 콘텐츠 보안 정책이 모든 네트워크 요청을 막는 HTML 파일 하나이므로 붙여 넣은 텍스트는 기기 밖으로 나가지 않습니다. 소스는 `web/index.html`, WebAssembly 크레이트는 `tools/laconic-web`입니다. 빌드 방법:

```bash
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown --manifest-path tools/laconic-web/Cargo.toml
cargo run --manifest-path tools/laconic-tools/Cargo.toml -- web-page tools/laconic-web/target/wasm32-unknown-unknown/release/laconic_web.wasm laconic.html
```

Linux(x86_64, 정적), Windows(x86_64), macOS(Apple silicon)용 사전 빌드 바이너리가 SHA-256 체크섬과 함께 각 `laconic-check-v*` 릴리스에 첨부됩니다. 바이너리를 PATH에 두세요. 소스에서 빌드하려면 Rust만 있으면 됩니다.

이미 [Vale](https://vale.sh)을 쓰는 프로젝트를 위해, `vale/Laconic`은 `lint`의 규칙 표에서 생성한 Vale 스타일이며 각 릴리스에 `Laconic.zip` 패키지로 첨부됩니다(0.6.0부터). 정규 표현식으로 나타낼 수 있는 규칙을 다룹니다. 나타낼 수 없는 7개와 나머지가 `lint`와 어떻게 다른지는 [vale/README.md](vale/README.md)에 있습니다.

0.4.0부터의 릴리스에는 각 아카이브의 빌드 출처 증명과 Scoop, Homebrew, winget, MCP Registry용 매니페스트도 있습니다. MCP 번들 `laconic-check.mcpb`는 Claude Desktop에 한 번에 설치됩니다. 내려받은 아카이브는 `gh attestation verify <archive> --repo yungSNAT/Laconic`으로 확인하세요. Scoop은 릴리스에서 바로 설치합니다.

```bash
scoop install https://github.com/yungSNAT/Laconic/releases/latest/download/laconic-check.json
```

Homebrew 포뮬러와 winget 매니페스트는 탭과 winget 카탈로그용으로 첨부되지만, 둘 다 아직 laconic-check를 등록하지 않았습니다.

[docs/rules.md](docs/rules.md)는 각 규칙을 이름별로 예시, Style Manual 해당 절, 사람이 쓴 산문을 얼마나 자주 지적하는지와 함께 설명하고, 검사기가 다루는 Style Manual 절도 보여 줍니다. 오탐은 [이슈 양식](https://github.com/yungSNAT/Laconic/issues/new?template=false-positive.yml)으로 알려 주세요. [docs/humanizer-coverage.md](docs/humanizer-coverage.md)는 `humanizer` 4.0.0의 패턴 26개 각각을 그것을 다루는 규칙과 산문 스킬 절에 대응시킵니다. 스킬은 26개 모두를 다루고, `lint`는 그중 8개를 어떤 형태로든 지적합니다.

`lint`를 CI나 pre-commit 훅의 관문으로 쓸 때, 다음 옵션이 무엇을 보고하고 무엇을 실패시킬지 정합니다.

| 옵션 | 효과 |
| --- | --- |
| `--format <format>` | 출력 형식을 고릅니다: `text`(기본값), `json`(`--json`과 같음) 또는 `sarif`(`--sarif`와 같음). |
| `--json` | `scope`('Checks house style, prose patterns and substance heuristics; never infers authorship.'), `findings`(각각 `file`, `line`, `column`, `rule`, `advisory`, `severity`, `confidence`, `message`, `url` 포함), `failing`, `suppressed`를 담은 JSON 객체 하나를 출력합니다. |
| `--sarif` | GitHub code scanning과 다른 SARIF 뷰어용 [SARIF 2.1.0](https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html) 로그를 출력합니다. 모든 규칙을 설명과 링크와 함께 담고, 각 지적을 `warning` 수준(조언이면 `note`) 결과로 담으며, 각 결과에는 `severity`와 `confidence` 속성이, 실행에는 `scope`가 붙습니다. 위치는 code scanning이 저장소 경로로 대응시키는 파일의 절대 `file` URI와, SARIF가 세는 방식대로 UTF-16 코드 단위의 열로 나타냅니다. `--json`과 함께 쓸 수 없으며, 0.6.0부터입니다. |
| `--metrics` | 지적 사항 뒤에 각 파일의 문장 지표를 출력하고, 파일이 2개 이상이면 전체 지표도 출력합니다. `--json`에서는 `files`와 `overall`을 담은 `metrics` 객체를 더합니다. `--sarif`와 함께 쓸 수 없으며, 0.6.0부터입니다. |
| `--profile <name>` | 글의 용도별 프리셋을 적용합니다: `email`, `report`, `docs`, `chat`, `essay`. `laconic.toml` 또는 `.laconic-check`의 `profile = <name>` 줄도 같은 일을 하며, `--profile`이 그보다 우선합니다. 0.6.0부터입니다. |
| `--disable <rules>` | 이 규칙들의 지적을 버립니다. 쉼표로 구분하며 반복할 수 있습니다. |
| `--only <rules>` | 이 규칙들의 지적만 남깁니다. `--disable`이 우선합니다. |
| `--max-findings <n>` | 실패 지적이 n개를 넘을 때만 실패합니다(기본값 0). |
| `--warn` | 지적 때문에 실패하지 않습니다. |
| `--commit-msg` | 각 파일을 Git 커밋 메시지로 읽으며, 주석 줄과 가위 줄 아래는 제외합니다. |
| `--config <file>` | `laconic.toml` 또는 `.laconic-check` 대신 이 파일에서 설정을 읽습니다. |

프로필은 어떤 규칙을 실행하고 어떤 규칙으로 실패할지를 바꿉니다. 프로필이 없으면 `lint`는 그대로입니다. `email`, `report`, `essay`는 [`unglossed-term`](docs/rules.md#unglossed-term)을 켭니다. 이 규칙은 'Full Name (ACR)'이나 'ACR stands for' 같은 설명이 앞에 없는 대문자 3~8자의 약어를 알리는 권고 규칙이며, 'BHP' 같은 널리 알려진 형태와 강조용 대문자 단어는 통과합니다. 모든 프로필은 다른 옵트인 규칙도 일부 켭니다. 어떤 규칙인지는 [docs/rules.md](docs/rules.md)의 각 규칙에 나옵니다. `--only unglossed-term`은 프로필 없이 이 규칙을 켭니다. 이 3개 프로필은 `stacked-hedge`로 실패하고, `email`과 `essay`는 `bold-label`, `essay`는 `clause-dashes`, `report`는 `hyphen-consistency`와 `acronym-before-definition`으로도 실패합니다. `chat`은 `stacked-hedge`, `bold-label`, `clause-dashes`로 실패하게 하고 `acronym-before-definition`과 `hyphen-consistency`를 뺍니다. `docs`는 `bold-label`과 `acronym-before-definition`을 빼고 `number-word`를 권고로 바꿉니다. 프로필은 5개의 옵트인 근거·채팅 상투어 규칙도 켭니다(0.6.0부터, 기본은 모두 권고): `email`은 `ai-placeholder`, `report`와 `essay`는 `unanchored-magnitude`, `unattributed-superlative`, `unnamed-evidence`, `ai-placeholder`, `docs`는 `unattributed-superlative`와 `ai-placeholder`, `chat`은 `ai-placeholder`를 켜고 `chat-filler`로 실패하게 합니다. 프로필 때문에 실패하게 된 지적은 메시지에서 '(advisory)'가 빠집니다.

```text
$ printf 'Each member gets a QBR plan. This could potentially slip a week.\n' > memo.md
$ laconic-check lint memo.md; echo "exit $?"
memo.md:1:35: stacked-hedge: stacked hedge 'could potentially' (advisory); keep one qualifier at the confidence the source supports
exit 0
$ laconic-check lint --profile report memo.md; echo "exit $?"
memo.md:1:35: stacked-hedge: stacked hedge 'could potentially'; keep one qualifier at the confidence the source supports
memo.md:1:20: unglossed-term: 'QBR' is used without an earlier gloss (advisory); expand it at first use, as in 'Full Name (QBR)' or 'QBR (Full Name)'
exit 1
$ printf 'The quarterly business review (QBR) sets targets. Each member gets a QBR plan.\n' > glossed.md
$ laconic-check lint --only unglossed-term memo.md glossed.md; echo "exit $?"
memo.md:1:20: unglossed-term: 'QBR' is used without an earlier gloss (advisory); expand it at first use, as in 'Full Name (QBR)' or 'QBR (Full Name)'
exit 0
```

`--metrics`는 문장만 측정하며 코드, URL, HTML 태그, 프런트 매터, 그리고 `> [!NOTE]` 같은 알림이 아닌 인용은 건너뜁니다. 단어 수, 문장 수, 평균 단어 길이, 긴 단어(7자 이상)의 비율, 명사화 단어(3자 이상의 어간 뒤에 '-tion', '-sion', '-ment', '-ness', '-ity', '-ance', '-ence'가 붙은 단어)의 비율, 'logs, metrics and traces' 같은 3항목 나열, 문장 길이의 평균과 중앙값을 보고합니다. 지적 사항이나 종료 상태는 바꾸지 않으므로 초안과 고쳐 쓴 글을 비교할 수 있습니다.

```text
$ printf 'We reviewed the logs, metrics and traces. The implementation of the measurement is complete.\n' > notes.md
$ laconic-check lint --metrics notes.md
metrics: notes.md: words=14 sentences=2 mean_word_len=5.43 long_words=35.7% nominalisations=14.3% triads=1 sentence_len=mean 7.0 / median 7.0
```

`lint`를 실행하는 디렉터리의 `laconic.toml` 또는 `.laconic-check` 파일(`laconic.toml` 우선)에는 `disable`, `only`, `profile`, `accept`, `max_findings`(`.laconic-check`에서는 `max-findings`)에 대한 `key = value` 줄과 `#` 주석 줄이 들어갑니다. `accept` 줄은 `us-spelling`이 지적하지 않을 글자만으로 된 단어를 나열하며, 각 단어는 같은 대소문자로 온전한 단어 단위로 대조됩니다. 그래서 `accept = color`여도 문장 첫머리의 'Color'는 계속 지적됩니다(0.6.0부터). `[protect]` 절은 일치하는 범위에서 지적을 면제하는 작성자 문구(`phrases = [...]` 또는 한 줄에 한 문구)를 나열합니다(0.6.0부터). 명령줄의 규칙은 목록에 더해지고, `--max-findings`는 개수를 대체합니다.

```text
# The CSS property names color and center are expected here.
accept = color, center
# This project uses serial commas.
disable = serial-comma
```

```toml
# laconic.toml uses TOML arrays, max_findings and [protect] phrase exemptions.
accept = ["color", "center"]
disable = ["serial-comma"]
max_findings = 0

[protect]
phrases = ["deep dive"]
```

설정 다음에 오는 `[rules]` 절에는 프로젝트 자체 규칙을 한 줄에 한 문구씩 씁니다. `id: phrase = replacement`는 대체어를 제안하고 `id: phrase =`는 문구를 금지합니다(0.6.0부터). 각 지적은 규칙을 `project/<id>`로 표시하며, `disable`, `only`, `--disable`, `--only`, 억제 주석이 이를 받습니다. 여러 줄이 같은 id를 쓸 수 있습니다. 문구는 글자와 숫자로 된 단어 하나 이상이며, 단어 안에는 하이픈이나 아포스트로피를 쓸 수 있습니다. 산문에서 온전한 단어 단위로, 줄바꿈을 넘어서도 대조하며, 코드, URL, 인용 안에서는 대조하지 않습니다. 소문자로 쓰면 대소문자에 상관없이 대조하고, 대문자가 있으면 쓴 그대로만 대조합니다. 대체어와 똑같이 쓴 텍스트는 지적하지 않으므로 `github = GitHub`는 'GitHub'를 그대로 둡니다. 프로젝트 지적도 Laconic 자체 지적처럼 `lint`를 실패시킵니다. 규칙 참조 링크는 없으며, `--json`에서 `url`은 `null`이고 SARIF 로그는 Laconic 규칙 뒤에 링크 없이 이 규칙들을 나열합니다. 형식이 잘못된 줄이나 알 수 없는 절이 있으면 `lint`는 그 줄 번호와 함께 멈춥니다.

```text
[rules]
plain-words: utilise = use
plain-words: leverage = use
filler: going forward =
names: Javascript = JavaScript
```

지적 하나를 끄려면 그 앞에 규칙 이름을 적은 주석을 한 줄로 두세요. 바로 다음의 비어 있지 않은 줄에만 적용되며, `lint`는 아무것도 끄지 않은 각 규칙 이름을 `unused-suppression`으로 보고합니다. 주석이 다른 규칙을 끄더라도 마찬가지입니다.

```markdown
<!-- laconic-check-disable-next-line us-spelling -->
Set the color property of the heading to red.
```

| 종료 상태 | 의미 |
| --- | --- |
| 0 | `lint`가 `--max-findings`가 허용하는 것보다 많은 실패 지적을 찾지 않았거나 `--warn`으로 실행됨. `preserve`가 빠진 것도, `--strict`에서 더해진 것도 찾지 않음. `mcp`가 입력 끝에 도달함. `lsp`가 `shutdown` 뒤에 `exit`를 받음. `hook`이 이벤트에 응답함. `--version`이나 `--help`가 텍스트를 출력함. |
| 1 | `lint`가 허용보다 많은 실패 지적을 찾음. `preserve`가 빠진 항목 또는 `--strict`에서 더해진 항목을 찾음. `lsp`가 `shutdown` 없이 `exit`를 받았거나 입력이 끝남. `hook`이 이벤트, 프로젝트 파일, 커밋하지 않은 Markdown 파일을 읽지 못했거나, Git이 저장소를 찾지 못한 경우 외의 이유로 실패함. |
| 2 | 사용법 오류. `hook` 외 명령의 프로젝트 파일 오류나 읽기 오류. 또는 `mcp`나 `lsp`의 입출력 오류. 읽기 오류가 나도 `lint`는 다른 파일을 계속 검사합니다(0.6.0부터). |

[pre-commit](https://pre-commit.com)으로 실행하려면 `laconic-check`를 PATH에 두고 훅을 추가하세요. 커밋 메시지 훅에는 `pre-commit install --hook-type commit-msg`도 필요하며, 0.3.0에는 `--commit-msg`가 없으므로 laconic-check 0.4.0 이상이 필요합니다.

```yaml
repos:
  - repo: https://github.com/yungSNAT/Laconic
    rev: <commit>
    hooks:
      - id: laconic-check
      - id: laconic-check-commit-msg
```

GitHub Actions에서는 복합 Action이 러너(Linux x86_64, Windows x86_64, Apple silicon의 macOS)에 맞는 laconic-check 릴리스를 내려받아 릴리스의 SHA-256 파일과 대조하고, 검사한 뒤 각 지적에 주석을 답니다. 체크섬은 손상되거나 바꿔치기된 다운로드를 잡아낼 뿐, 침해된 릴리스를 잡아내지는 못합니다.

```yaml
- uses: actions/checkout@v7
- uses: yungSNAT/Laconic@<commit>
  with:
    version: 0.6.1     # the laconic-check release; 0.6.1 by default
    files: README.md   # one path a line; default: every .md and .markdown file Git tracks
    args: --warn       # further lint options
```

대신 GitHub code scanning에 지적을 나열하려면 `laconic-check` 0.6.0 이상을 PATH에 두고, 작업에 `security-events: write`를 주고 로그를 올리세요. `--warn`이면 지적 때문에 단계가 실패하지 않으므로 사용법 오류나 읽기 오류에서만 실패하고, 그때는 불완전한 것이 올라가지 않습니다. 지적 때문에도 작업을 실패시키려면 위의 Action이나 `--warn` 없는 다른 `lint` 단계를 추가하세요.

```yaml
- run: laconic-check lint --sarif --warn README.md docs/*.md > laconic-check.sarif
- uses: github/codeql-action/upload-sarif@v4
  with:
    sarif_file: laconic-check.sarif
```

`lint -`는 표준 입력을 읽으므로 에이전트는 초안을 저장하지 않고도 검사할 수 있습니다. `laconic-check`가 PATH에 있으면 두 스킬 모두 작성한 결과물을 보내기 전에 그것을 실행합니다. 미국식에서 호주식으로 바꾸는 단어 목록은 손으로 쓴 목록과, [English Speller Database](https://github.com/en-wl/wordlist)에서 생성해 Style Manual에 맞춰 검토한 약 2,400쌍을 합친 것입니다(`laconic-tools gen-au-words`). 심은 결함 테스트는 깨끗한 산문에 규칙마다 알려진 결함을 하나씩 심고, 검사기가 각각을 정확히 한 번 잡되 깨끗한 텍스트에서는 아무것도 지적하지 않도록 요구합니다. 보정 테스트는 2018년과 2021년에 발간된 Productivity Commission 보고서 3건(사람이 쓴 산문 약 198,500단어, CC BY 3.0 AU)에 `lint`를 실행하고, 지적 2,246개, 조언 67개(절 대시 문단 41개, 겹친 얼버무림 8개, 정의 전에 쓴 약어 14개, 붙여 쓰기와 하이픈 쓰기가 섞인 합성어 4개), 또는 그 가운데 상투 문구, 꾸민 대조, 불필요한 하이픈 7개가 늘면 실패합니다. 이것이 사람이 쓴 글에서 지적이 순증하는 것을 막습니다. 2026년 11월 5일 외부 실행은 [jenna-russell/human_detectors](https://github.com/jenna-russell/human_detectors)의 커밋 `afcf03d`에 있는 사람이 쓴 기사 150개(대부분 미국식 논픽션 110,723단어, 여기에 저장하지 않음)를 검사했습니다. 1,000단어당 지적 25.9개였고, 97%가 Laconic 하우스 스타일과의 차이(큰따옴표, 미국식 철자, 엠 대시, 글자로 쓴 숫자, 단어 선택, 연속 쉼표)였습니다. AI 흔적 규칙은 18번 작동했으며(1,000단어당 0.16: 꾸민 대조 8, 상투 문구 7, 폭로형 질문 2, 챗봇식 맺음말 1), 아직 참과 거짓으로 분류하지 않았습니다. 기사가 일반 텍스트이므로 이 실행은 `bold-label`을 시험하지 않습니다.

2026년 11월 13일, [AI Humanizer Benchmark](https://huggingface.co/datasets/ai-humanizer-benchmark/ai-humanizer-benchmark) 2026-10 주기에서 상용 휴머나이저 11개가 만든 재작성 363개에 `preserve`를 실행하고, 그 결과를 논문 [Humanized but Still True?](https://arxiv.org/abs/2611.00359)와 함께 공개된 문서별 사실 판정과 비교했습니다. 고정한 리비전과 명령은 [evals/README.md](evals/README.md#fact-audit-agreement)에 있습니다.

| 논문의 문서 분류 | 문서 수 | laconic-check 0.4.0 | laconic-check 0.5.0 | `--loose`를 쓴 `main` |
| --- | --- | --- | --- | --- |
| 단위가 하나라도 바뀜 | 105 | 105 실패 | 105 실패 | 105 실패 |
| 숫자가 바뀜 | 47 | 44 빠진 숫자나 양 보고 | 43 | 43 |
| 3단어 이상 인용이 바뀜 | 58 | 58 빠진 인용 보고 | 58 | 58 |
| 이름이 바뀜 | 28 | 20 확인할 이름 나열 | 20 | 20 |
| 모든 단위 유지 | 258 | 17 통과, 224 의미 단어만으로 실패, 0 월·인라인 코드·URL로 실패, 17 숫자·양·인용으로 실패 | 23 통과; 222; 0; 13 | 64 통과; 182; 0; 12 |

같은 문서에서 avoid-ai-writing의 `validate.js`는 바뀐 105개 중 23개를, Zero Slop의 충실도 검사는 105개 모두를 실패시킵니다. 유지된 258개 중 각각 233개와 15개를 통과시키며, `preserve`는 23개, `--loose`는 64개입니다([evals/README.md](evals/README.md#against-other-fact-checkers), 2026년 11월 16일). 논문의 표 1에 따르면 각 휴머나이저는 단위가 있는 재작성 텍스트 20개 중 7~13개(35%~65%, Undetectable AI는 20개 중 11개)에서 단위를 하나 이상 바꿨습니다. 각각에 대한 `preserve` 결과가 담긴 표는 [evals/README.md](evals/README.md#by-humaniser)에 있습니다.

`laconic-check mcp`는 `lint`와 `preserve`를 표준 입출력으로 MCP 도구로 제공하며, 따로 설치할 것이 없습니다. `preserve` 도구는 `--loose`와 같은 동작을 위해 `loose: true`를 받고, `lint` 도구는 `lint`처럼 서버가 실행되는 디렉터리의 `laconic.toml` 또는 `.laconic-check` 파일을 호출마다 다시 읽어 적용합니다. 형식이 잘못된 파일이면 호출이 그 줄을 알려 주는 오류가 됩니다(둘 다 0.6.0부터).

```bash
claude mcp add --scope user laconic-check -- laconic-check mcp
```

laconic-check 0.3.0 이상에 들어 있습니다.

`laconic-check lsp`는 표준 입출력으로 동작하는 언어 서버로, 입력하는 동안 편집기가 `lint`의 지적을 보여 줍니다(0.6.0부터). 열린 Markdown 문서가 바뀔 때마다 검사하고, 각 지적을 경고로(조언이면 힌트로) 규칙에 링크해 표시합니다. 문서를 담은 작업 공간 폴더(폴더가 중첩되면 가장 안쪽)의 `laconic.toml` 또는 `.laconic-check` 파일을 적용하며, 읽거나 해석할 수 없는 파일은 첫 줄에 오류로 표시합니다. 0.6.0부터는 규칙이 단 하나의 대체어를 아는 곳마다 빠른 수정도 제공합니다. 미국식 철자, 단어로 쓴 숫자, 라틴어 약어, 시각이나 날짜 형식, 큰따옴표, 보이지 않는 문자 등이 그렇습니다. 수정은 지적된 텍스트만 바꾸며, 직접 고를 때만 적용됩니다(Neovim에서는 `gra`, Helix에서는 `space a`). 엠 대시, 상투적 표현, 연속 쉼표 후보처럼 고쳐 써야 하는 규칙은 수정을 제공하지 않습니다. 수정은 문서의 버전을 함께 알리므로, 텍스트가 바뀐 뒤에 도착한 수정은 편집기가 엉뚱한 곳에 적용하지 않고 거부합니다. 이런 편집을 받는다고 알리지 않는 편집기는 수정을 받지 못하며, 알리지 않고도 받는 Neovim만 예외입니다. Neovim은 문서가 처음 바뀌기 전에 만든 수정은 확인하지 않으므로, 문서가 열린 뒤 한 번이라도 바뀐 다음부터만 수정을 받습니다. `laconic.toml`, `.laconic-check`나 `.git`이 있는 폴더를 작업 공간 폴더로 삼는 Neovim 0.11 이상에서는:

```lua
vim.lsp.config('laconic_check', {
  cmd = { 'laconic-check', 'lsp' },
  filetypes = { 'markdown' },
  root_markers = { 'laconic.toml', '.laconic-check', '.git' },
})
vim.lsp.enable('laconic_check')
```

Helix의 `languages.toml`에서는 이 목록이 Helix 기본 Markdown 서버를 대체하므로, 쓰는 다른 서버도 추가하세요.

```toml
[language-server.laconic-check]
command = "laconic-check"
args = ["lsp"]

[[language]]
name = "markdown"
language-servers = ["laconic-check"]
```

`laconic-check hook`은 Claude Code와 Codex용 Stop 훅이며, 추가하기 전까지는 꺼져 있습니다(0.6.0부터). 에이전트가 차례를 끝내려 할 때, 세션 디렉터리를 담은 저장소의 커밋하지 않은 Markdown(스테이징됨, 스테이징 안 됨, 추적 안 됨)을 저장소 최상위의 `laconic.toml` 또는 `.laconic-check` 파일로 검사합니다. `lint`가 실패할 상황이면 에이전트에게 실패 지적을 먼저 살펴보라고 요청하고 최대 50개를 나열합니다. 차례마다 한 번만 요청하므로 지적이 오탐일 때도 에이전트는 끝낼 수 있습니다. 에이전트의 편집만이 아니라 사용자 자신의 커밋하지 않은 편집도 검사합니다. Claude Code에서는 `.claude/settings.json`이나 `~/.claude/settings.json`에 다음을 추가하세요. Codex는 같은 블록을 `~/.codex/hooks.json`에서 읽으며, 새 훅은 `/hooks`에서 검토할 때까지 신뢰하지 않음으로 표시합니다.

```json
{
  "hooks": {
    "Stop": [
      { "hooks": [{ "type": "command", "command": "laconic-check hook", "timeout": 60 }] }
    ]
  }
}
```

<a id="always-on"></a>
## 항상 켜기

모든 Claude Code 세션에서 간결 규칙을 로드하려면 규칙 폴더에 복사하세요.

```bash
mkdir -p ~/.claude/rules && cp rules/laconic.md ~/.claude/rules/laconic.md
```

대신 모든 Claude Code 세션을 전체 세션 모드로 시작하려면 빈 플래그 파일을 만드세요.

```bash
touch ~/.claude/.laconic-always
```

세션 모드를 끄려면 파일을 지우세요. 둘 중 하나만 고르세요. 세션 모드에 이미 간결 규칙이 들어 있으므로 둘 다 로드하면 세션마다 약 240단어가 반복됩니다. 다른 에이전트는 [INSTALL.md](INSTALL.md)에서 다룹니다.

<a id="development"></a>
## 개발

Codecov 배지는 `tools/laconic-evals`의 평가 도구 `laconic-evals`를 측정합니다. 다른 Rust 검사는 CI에서 실행됩니다.

```sh
cargo test --manifest-path tools/laconic-tools/Cargo.toml
cargo test --manifest-path tools/laconic-evals/Cargo.toml
cargo run --quiet --locked --manifest-path tools/laconic-evals/Cargo.toml -- validate
cargo test --manifest-path tools/laconic-check/Cargo.toml
```

검증, 평가, 릴리스 관문은 [CONTRIBUTING.md](CONTRIBUTING.md)에서 다룹니다.

<a id="credits"></a>
## 크레딧

Laconic은 Ayoub G.의 [i-have-adhd](https://github.com/ayghri/i-have-adhd)(MIT)를 포크한 것으로, 응답 규칙, 플랫폼 어댑터, 훅, 평가 도구가 여기서 왔습니다. 글쓰기 안내는 Peter Yang의 No AI Slop과 Hardik Pandya의 Stop Slop을 바탕으로 한 No AI Slop에서 왔으며, [humanizer](https://github.com/blader/humanizer), [avoid-ai-writing](https://github.com/conorbronsdon/avoid-ai-writing), [Sepia](https://github.com/Nanako0129/sepia)(모두 MIT)에서 가져와 고친 내용이 있습니다. 호주 영어 규칙은 [Australian Government Style Manual](https://www.stylemanual.gov.au/)(copyright Commonwealth of Australia)을 따르되 Laconic의 말로 다시 썼습니다. 이 매뉴얼은 공개 라이선스로 발행되지 않았으며, Laconic은 그 사본을 포함하지 않습니다. `laconic-check` 보정 말뭉치는 [Productivity Commission](https://www.pc.gov.au/) 보고서 3건(CC BY 3.0 AU)의 텍스트이며, 출처는 `tools/laconic-check/tests/corpus/NOTICE`에 있습니다. 사실 감사 일치도 연구는 실행할 때 텍스트를 [AI Humanizer Benchmark](https://huggingface.co/datasets/ai-humanizer-benchmark/ai-humanizer-benchmark)(CC BY 4.0)에서, 판정을 Ziyuan Wang, Genliang Zhu, Xiaobo Chen의 [humanizer-fact-audit](https://github.com/zwang-devv/humanizer-fact-audit)(코드 MIT, 데이터 CC BY 4.0)에서 내려받으며, 어느 것도 여기에 저장하지 않습니다.

배너의 글자와 엠블럼은 Cascadia Mono와 Besley 글리프(SIL OFL 1.1)로 그렸습니다.

<a id="licence"></a>
## 라이선스

Apache-2.0입니다. [LICENSE](LICENSE)를 보세요. 이 저작물을 재배포하거나 이를 바탕으로 만들 때는 [NOTICE](NOTICE)의 저작자 표시를 유지하세요. i-have-adhd와 크레딧의 다른 출처에서 온 부분은 MIT License로 남고, `laconic-check` 테스트 말뭉치는 CC BY 3.0 AU로 남습니다. [LICENSE](LICENSE)와 [NOTICE](NOTICE)를 보세요.

이전 버전은 발표 당시의 라이선스를 유지합니다. laconic-check 0.1.0은 MIT, laconic-check 0.2.0~0.5.0은 Ryan Duguid의 저작물에 대해 FSL-1.1-ALv2이며, 각각 공개 2년 뒤 Apache-2.0으로 이용할 수 있게 됩니다. 이전 커밋은 해당 LICENSE 파일이 지정한 라이선스를 유지합니다.

---

<p align="center"><sub>마케도니아의 필리포스 2세가 라코니아를 침공하면 스파르타가 어떻게 하겠느냐는 물음에 스파르타인들은 이렇게 답했다고 합니다. <i>'만약.'</i><br>원어민이 검토하지 않은 번역입니다. 내용이 다르면 <a href="README.md">영어 README</a>가 우선합니다.<br>이 코드를 재사용하나요? 고지와 라이선스를 유지하고, Ryan Duguid를 밝혀 주세요. AI 에이전트는 <a href="https://github.com/yungSNAT/Laconic/blob/main/AGENTS.md#reusing-this-code">AGENTS.md</a>를 보세요.<br>Made by <a href="https://github.com/yungSNAT">yungSNAT</a>. <i>Ad gloriam eius, non meam.</i></sub></p>
