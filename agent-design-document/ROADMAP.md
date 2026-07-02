# Product Roadmap va Execution Checklist
# AgentForge AI

> **Trang thai:** Ke hoach de trien khai, khong phai danh sach tinh nang da hoan thanh
> **Cap nhat:** 2026-06-26
> **Muc tieu:** Bien cac nang luc orchestration, governance va multi-agent hien co thanh mot trai nghiem lam viec thong nhat.

---

## 1. Diem xuat phat

Codebase da co nhung khoi nen quan trong:

- Team Workspace, Team Bus va Virtual Office.
- Slash command registry: `/goal`, `/spec`, `/plan`, `/run`, `/review`, `/fix`, `/test`, `/verify`, `/help`.
- iFlow co `AgentTask`, `Decision`, `HumanReview`, `CronTrigger`, version va persisted execution state.
- Orchestration run, run event, approval request, tool invocation va artifact deu co persistence.
- Research Notebook, Knowledge, Monitoring, provider abstraction va governed learning pipeline.

Roadmap nay uu tien dong goi cac khoi san co thanh cac luong cong viec ro rang. Khong mo rong schema hoac tao subsystem moi neu du lieu hien co da dap ung duoc use case.

## 1.1 Implementation update - 2026-06-26

- [x] Them selection state cho `orchestration_run` va persist run duoc chon trong settings.
- [x] Them Run Workspace trong Orchestration: overview, token/task metrics, event timeline, artifacts va pending approvals theo `run_id`.
- [x] Them deep-link "View run" tu Orchestration Dashboard va nut "Run" tu Team Chat khi co iFlow run.
- [x] Them Office Agent Inspector co agent selection, status va active task count; canvas click va sidebar click deu chon duoc agent.
- [x] Artifact preview, add-to-Knowledge va action approval chi tiet trong Run Workspace/Artifact Hub.
- [ ] Run Workspace hien tai dung layout theo section; chi tach thanh tab rieng khi timeline/artifact du lon de can dieu huong.

## 1.2 Implementation update - 2026-06-28

- [x] Artifact Hub co file status, preview text/markdown, hash warning, copy path, copy hash, open producing run va "Add to Knowledge".
- [x] "Add to Knowledge" dung `source_uri_normalized` de upsert artifact cung nguon, tranh import trung khi content hash khong doi.
- [x] Run Workspace hien thi approval detail: tool, mode, path/command, risk label, requested_by va approve/reject tai cho.
- [x] Approval action trong Run Workspace tai su dung cung luong Governance: resolve request, update waiting tasks/run status, insert run event/audit log va resume/reject iFlow.
- [x] Office Agent Inspector hien current run/task khi agent co task active va co nut mo Run Workspace.
- [x] OS reveal/open file path, artifact kind filter va timeline filter da co trong Run Workspace/Artifact Hub.

## 2. Nguyen tac thuc hien

- **Run-first:** Moi hanh dong co tac dong phai quy ve mot `orchestration_run` hoac workflow execution co the truy vet.
- **Governed by default:** Tool mutation, handoff va phe duyet phai di qua Tool Execution Gateway; UI khong duoc bo qua policy.
- **Artifact-first:** Spec, plan, review, test result va deliverable phai co nguon, hash, run va duong dan ro rang.
- **Progressive disclosure:** Chat la diem vao nhanh; Mission Control mo khi cong viec can theo doi chi tiet.
- **No fake intelligence:** Ket qua research phai co citation; semantic search phai dung embedding that truoc khi duoc goi la semantic.
- **Backward compatible:** Cac slash command va `/run` hien co phai tiep tuc hoat dong trong suot qua trinh nang cap.

## 3. Thu tu va cac luong song song

| Wave | Luong chinh | Co the lam song song | Phu thuoc |
|---|---|---|---|
| A | Mission Control va Artifact Hub | Virtual Office Inspector, tai lieu | Khong |
| B | Goal Intake va Workflow Templates | Research Evidence | Artifact Hub co ban |
| C | Budget va Smart Routing | Research Evidence tiep tuc | Data contract cho token/cost |
| D | Governed Learning Experience | Polish Virtual Office | Mission Control, Artifact Hub |

**Nguyen tac merge:** Moi wave phai co feature flag hoac empty state ro rang, migration forward-only, va `cargo fmt` + `cargo check` truoc khi merge.

---

## Wave A - Mission Control va Artifact Hub

### A1. Run Workspace

**Muc tieu:** Mo mot run nhu mot khong gian lam viec, thay vi buoc nguoi dung phai ghep chat, Orchestration va iFlow bang tay.

**Tasks**

- [ ] Tao `RunWorkspace` state nhan `run_id`, co lazy reload tu `DatabasePort`.
- [ ] Tao thanh dieu huong gom `Overview`, `Timeline`, `Workflow`, `Artifacts`, `Approvals`.
- [x] Hien thi thong tin goal, instance, mode, status, thoi gian tao/cap nhat va agent khoi tao.
- [x] Chuan hoa `RunEventRecord.event_type` thanh nhan, severity/mau va mo ta de doc.
- [x] Them filter Timeline theo event category doc tu `event_type`.
- [ ] Them filter Timeline theo agent, task va khoang thoi gian neu run co du event metadata.
- [ ] Mo iFlow Builder tai workflow version cua run, o read-only mode neu run da dispatch.
- [ ] Them deep-link tu Team Chat, Orchestration panel, Monitoring va Office Inspector vao Run Workspace.

**Backend/data checklist**

- [ ] Su dung `list_recent_run_events(Some(run_id), ...)` hoac method tuong duong, khong doc log tu UI cache.
- [ ] Xac dinh va document event taxonomy toi thieu: created, dispatched, agent_started, tool_allowed, waiting_approval, artifact_created, completed, failed, cancelled.
- [ ] Them pagination/debounce neu run co nhieu hon 200 events.

**UX checklist**

- [ ] Trang thai dang chay, cho duyet, thanh cong, that bai, cancelled co mau va icon nhat quan.
- [ ] Empty state huong dan khi run chua co event/artifact.
- [ ] Nut copy `run_id` va open related workflow/agent co tooltip.
- [ ] Khong dung card long trong card; cac tab la khu vuc day du chieu rong.

**Acceptance criteria**

- [ ] Tu mot message hoac office agent, nguoi dung mo duoc dung Run Workspace trong toi da hai thao tac.
- [ ] Timeline sap xep on dinh theo timestamp va khong mat event sau khi restart app.
- [ ] Chuc nang xem run khong sua workflow state hoac tool approval state.

### A2. Artifact Hub

**Muc tieu:** Bien artifact da persist thanh dau ra co the xem, tai su dung va dua vao Knowledge.

**Tasks**

- [x] Render danh sach `ArtifactRecord` theo kind, agent, thoi gian va content hash.
- [x] Preview Markdown/text trong app; hien thi trang thong tin file cho binary/docx/pdf khi chua co viewer native.
- [x] Them action copy path, copy hash, open producing run va "Add to Knowledge".
- [x] Them OS reveal/open path neu can thao tac voi file ngoai app.
- [x] Them filter artifact kind tu `ArtifactRecord.artifact_kind`.
- [ ] Ket noi output cua slash command vao artifact list ngay sau khi workflow tao file.
- [x] Hien thi warning khi artifact path khong con ton tai hoac hash thay doi.

**Acceptance criteria**

- [ ] `/spec`, `/plan` va `/review` tao artifact co the tim thay tu Run Workspace.
- [x] Cung mot artifact khong duoc import trung vao Knowledge neu content hash khong doi.
- [x] Failed preview khong lam crash panel va van giu metadata artifact.

**Kiem thu**

- [ ] Unit test formatter cho event taxonomy va artifact kind.
- [ ] Integration test persist -> reload -> display cho run event/artifact.
- [ ] Manual test voi file missing, Unicode path va artifact lon.

---

## Wave A - Virtual Office Inspector (song song A1/A2)

### A3. Agent Inspector va Office as Command Center

**Muc tieu:** Virtual Office la surface quan sat va dieu huong cong viec, khong chi la minh hoa.

**Tasks**

- [x] Them `selected_office_agent_id` vao `TeamWorkspacePanel`; click agent de chon, drag van giu hanh vi rieng.
- [ ] Them side inspector hoac detail drawer: role, status, current task, related run, token usage, message gan nhat va health.
- [ ] Mapping status -> office state: planning, working, waiting approval, blocked, idle, offline.
- [ ] Them visual cue tiet che: outline, status dot, task progress va tooltip khi hover.
- [ ] Them actions: open chat with agent, open current run, view task, request status update.
- [ ] Tinh toan room occupancy tu task/run state thay vi chi tu status string.

**Khong lam o wave nay**

- [ ] Khong persist keo tha toa do; day la preference UI, chi them sau khi co use case ro rang.
- [ ] Khong dung Virtual Office de thay the bang task board.

**Acceptance criteria**

- [ ] Click mot agent dang lam task mo dung task/run hoac empty state co ly do.
- [ ] Keyboard va mouse drag khong xung dot voi agent selection.
- [ ] Layout van ro rang o viewport nho va khong de text de len sprite.

---

## Wave B - Goal Intake va Workflow Templates

### B1. Goal Intake / Mission Studio

**Muc tieu:** `/goal` tro thanh mot luong intake co cau truc, giup tao cong viec dung ngay tu dau.

**Tasks**

- [ ] Sau `/goal`, hien thi Goal Brief co objective, scope, constraints, acceptance criteria, stakeholder, deadline va risk.
- [ ] Cho phep agent de xuat assumptions; nguoi dung accept/edit truoc khi dispatch.
- [ ] Luu Goal Brief nhu artifact Markdown va link voi `run_id`.
- [ ] Cho chon execution mode va team instance truoc khi chay.
- [ ] De xuat template workflow phu hop dua tren loai goal; nguoi dung co the chon, xem truoc hoac chay prompt-only.
- [ ] Them action "Convert to /spec" va "Convert to /plan" de tai su dung context cua cung run.

**Acceptance criteria**

- [ ] `/goal` voi mot cau ngan co the tao brief du thong tin hoac hoi dung cau hoi bo sung.
- [ ] Muc tieu da xac nhan phai xuat hien trong run, workflow context va artifact.
- [ ] Unknown slash command van bao loi va goi y `/help`, khong gui nhu chat thuong.

### B2. Template Gallery va Scheduling

**Muc tieu:** Tai su dung iFlow thay vi bat nguoi dung tao workflow tu trang.

**Mau ban dau**

- [ ] Feature Delivery: goal -> spec -> plan -> implement -> test -> review.
- [ ] Bug Triage: reproduce -> diagnose -> fix -> test -> verify.
- [ ] Research Brief: research -> source review -> synthesis -> approval -> export.
- [ ] Release Readiness: collect changes -> tests -> review -> release notes -> human review.
- [ ] Daily Health Check: cron -> collect metrics -> summarize -> notify.

**Tasks**

- [ ] Dinh nghia template JSON da validate, versioned va read-only o gallery.
- [ ] Clone template thanh workflow scope theo instance truoc khi edit/chay.
- [ ] Them template preview: node graph, required agent roles, tool permissions, du kien artifact.
- [ ] Them scheduling UI cho `CronTrigger`: interval, timezone behavior, enable/disable, last run, next run.
- [ ] Ghi event khi scheduled run khong dispatch duoc va hien thi o Monitoring.

**Acceptance criteria**

- [ ] Template clone khong lam thay doi ban goc.
- [ ] Scheduled workflow khong chay khi deactivated va resume dung sau restart.
- [ ] Template co role thieu phai bao loi truoc dispatch.

---

## Wave B/C - Research Evidence va Knowledge Chat

### C1. Research Evidence

**Muc tieu:** Phan biet ro rang noi dung tong hop va bang chung nguon.

**Tasks**

- [ ] Luu citation theo tung claim: title, URL, snippet, fetched_at, source type va confidence.
- [ ] Render source card co open URL, copy citation va show exact supporting excerpt.
- [ ] Them action "Challenge conclusion" tao mot review run voi prompt phan bien.
- [ ] Khi save notebook, tach Research Summary va Sources thay vi mot blob Markdown.
- [ ] Ghi provenance khi mot spec/plan su dung notebook lam input.

### C2. Semantic Search that

**Tasks**

- [ ] Thay mock embedding bang `fastembed` da co trong dependencies hoac provider embedding da cau hinh.
- [ ] Chon embedding model, dimension va version; persist metadata de co the reindex.
- [ ] Chunk document theo heading/paragraph, luu source span va content hash.
- [ ] Ket hop FTS + vector score + freshness + trust level trong hybrid ranking.
- [ ] Them Reindex va progress UI, chay o background.

**Acceptance criteria**

- [ ] Hai query khac nhau khong the nhan cung mot mock vector.
- [ ] Ket qua search hien citation va doan ngu canh, khong chi title.
- [ ] Reindex co the resume an toan sau khi app restart.

---

## Wave C - Budget, Cost va Smart Routing

### D1. Cost Data Contract

**Tasks**

- [ ] Dinh nghia pricing record theo provider/model, input/output token, currency va effective date.
- [ ] Them database migration forward-only cho budget policy va cost usage; backfill cost = unknown cho du lieu cu.
- [ ] Ghi token va estimated/actual cost theo invocation, run, agent va instance.
- [ ] Them cap theo run, instance va month; policy block/warn/ask approval khi cham nguong.
- [ ] Thay "Estimated Cost (MTD): N/A" bang gia tri, unknown state hoac thong bao can pricing config.

### D2. Smart Provider Routing

**Tasks**

- [ ] Tach provider selection policy: capability, model health, latency, quota, priority, cost va privacy constraint.
- [ ] Hien thi "Why this model" truoc khi dispatch trong supervision mode.
- [ ] Record fallback chain va failure reason vao run timeline.
- [ ] Cho phep pin provider/model per goal de tranh routing bat ngo.

**Acceptance criteria**

- [ ] Chi phi tong cua run bang tong invocation cost trong sai so lam tron da document.
- [ ] Budget block khong tao mutation pending dang do va co event truy vet.
- [ ] Fallback khong lam mat context snapshot hoac tool approval.

---

## Wave D - Governed Learning Experience

### E1. Feedback tu Run va Artifact

**Tasks**

- [ ] Them feedback nhanh tren completed run va artifact: useful, incorrect, incomplete, unsafe, other.
- [ ] Yeu cau comment cho feedback negative va lien ket dung subject/run/artifact.
- [ ] Hien thi feedback trong Learning tab voi evidence, owner va status.
- [ ] Tu feedback da validate, tao lesson va candidate skill/workflow qua dialog co diff preview.

### E2. Candidate Review

**Tasks**

- [ ] Hien thi baseline vs candidate definition diff.
- [ ] Hien thi benchmark score, canary cohort, promotion decision va rollback reason tren mot timeline.
- [ ] Chi cho promote khi candidate da qua benchmark va canary; giu rule nay o backend.
- [ ] Tao audit event cho moi transition.

**Acceptance criteria**

- [ ] Khong co candidate nao duoc promote truc tiep tu draft.
- [ ] Feedback va lesson co source evidence ro rang.
- [ ] Rollback dua workflow/skill ve baseline da luu va hien thi ly do.

---

## 4. Definition of Done chung

- [ ] UX co loading, empty, error, offline va permission-denied state.
- [ ] Accessibility: tooltip cho icon-only button, keyboard navigation, focus ro rang va text khong tran.
- [ ] Logging: event co `run_id`, actor va payload an toan; khong log secret/raw sealed payload.
- [ ] Database: migration forward-only, idempotent, co test upgrade cho SQLite database cu.
- [ ] Tests: unit cho parser/mapper/policy; integration cho persistence va resume; manual scenario theo use case.
- [ ] Quality gate: `cargo fmt`, `cargo check`, targeted tests khong bi treo.
- [ ] Documentation: cap nhat PRD/SRS/Technical Spec/Database Design cung PR; danh dau ro "implemented" hoac "planned".

## 5. Tai lieu can cap nhat theo milestone

| Milestone | Tai lieu bat buoc |
|---|---|
| A1/A2 | PRD, SRS, Technical Spec, Database Design neu co schema, UI Design |
| A3 | UI Design, Technical Spec |
| B1/B2 | PRD, SRS, Technical Spec, DIAGRAMS |
| C1/C2 | PRD, SRS, Database Design, Security notes |
| D1/D2 | PRD, SRS, Database Design, provider configuration guide |
| E1/E2 | PRD, SRS, Technical Spec, governance/audit documentation |

## 6. Backlog khong uu tien truoc

- Persist toa do keo tha trong Virtual Office.
- Mo phong pixel art phuc tap hoac animation khong mang them thong tin van hanh.
- Auto-promotion cho skill/workflow khong co benchmark va canary.
- Tu dong tin chi phi khi provider/model chua co pricing configuration.
- Goi semantic search neu embedding dang la mock.
