# Công việc ctdozlite đã tạm dừng — 01/11/2026

## Yêu cầu cần tiếp tục

1. Tự đồng bộ tin nhắn cũ. Người dùng xác nhận lịch sử có cả trên điện thoại và Web/PC, **ưu tiên lấy từ điện thoại**.
2. Hỗ trợ gõ tiếng Việt qua IBus Bamboo.
3. Hiển thị tên nhóm thật thay cho “Nhóm + ID”.
4. Hiển thị giờ theo GMT+7 trong danh sách hội thoại và nội dung chat.

## Bản ổn định và vị trí mã nguồn

- Bản đang cài: 0.1.1, binary `/home/triskim/.local/bin/zalo-rust`; chưa thay bằng bản sửa mới.
- Repository: `/home/triskim/Documents/Codex/2026-11-01/kie/work/ctdozlite`.
- Base đã đẩy lên GitHub: `c105e6871121d19375a038cb8c2481bd975fa3bd`.
- Thay đổi mới lưu trên nhánh cục bộ `wip/history-ibus-groups-vntime`; chưa đẩy lên GitHub.

## Đã làm và bằng chứng

- Sửa lấy metadata nhóm: ép version=0 và chia lô 25 nhóm. Kiểm tra đọc trực tiếp trên tài khoản trả 215 nhóm, 208 nhóm có tên được nhận diện ở thời điểm kiểm tra. Không in tên nhóm, ID, nội dung hay khóa vào log chẩn đoán. Sau đó đã sửa thêm nhận diện nhãn tạm để tên thật bắt đầu bằng “Nhóm ” không bị coi là tên tạm; chưa chạy lại toàn bộ kiểm tra sau sửa cuối này.
- Thêm hàm định dạng GMT+7, dùng cho parse tin nhắn, hội thoại, lịch sử SQLite và cache cũ. Kiểm thử ngày/giờ đã qua trong lần chạy 115 tests.
- Thêm tự tải lịch sử có trên máy chủ ở nền, phân trang có chặn cursor lặp, giữ mã hóa lịch sử, không đánh dấu lịch sử vừa tải là tin chưa đọc. Đây là backfill từ máy chủ, **không phải đồng bộ trực tiếp từ điện thoại**.
- Thêm chặn Enter gửi tin khi IME đang soạn/commit, launcher thiết lập XIM/IBus.
- Tìm thấy egui-winit 0.29.1 bỏ qua IME trên Linux. Đã vendor crate, giữ giấy phép, áp dụng backport upstream PR #5198 / merge commit `3ffe1ed7742a8927caee818c49018f59c93b0615`.
- Bộ test thư viện gần nhất: 115 pass, 3 ignore. Lần này chạy trước một số sửa cuối và backport; không coi là xác nhận toàn bộ trạng thái WIP.

## Phần chưa xong

- **Đồng bộ trực tiếp từ điện thoại chưa triển khai.** Client Zalo hiện tại dùng sync-v2, request 590 / wake-up 592, phiên riêng, khóa identity/ephemeral và dữ liệu batch mã hóa (crypto WASM riêng). Có endpoint HTTP `/api/transfer-sync-v2/request-sync` và các endpoint legacy `pull_mobile_msg`, `get_crossdb`, nhưng chưa đủ receiver/decryptor để triển khai đúng; chưa gửi yêu cầu chuyển lịch sử giả hoặc không thể giải mã.
- Native IME probe đã chạy hai lần và chưa đạt: trước backport không nhận IME, chỉ có dấu cách; sau backport lần thử vẫn không nhận được chuỗi mẫu, ghi 1 sự kiện IME. Đã sửa probe ghi chi tiết sự kiện và thao tác click focus, compile thành công nhưng **chưa chạy lại lần thử mới nhất** do người dùng yêu cầu dừng.
- Cần rà soát giới hạn phân trang 200 trang/hội thoại, việc tiếp tục sau ngắt, hiệu năng ghi lại toàn bộ thread mỗi trang và kiểm thử trực tiếp pagination trước khi coi backfill hoàn tất.
- Chưa cập nhật số phiên bản, chưa build/phát hành/cài bản mới, chưa commit vào main hoặc đẩy WIP lên GitHub.

## Điểm tiếp tục

1. Tiếp tục native probe ngoại tuyến, không gõ hay gửi tin thử vào hội thoại thật. `src/bin/ime_probe.rs`, `/home/triskim/Documents/Codex/2026-11-01/kie/work/type_ime_probe.py`, `work/ime-result.json`, `work/ime-build.log`.
2. Xác minh focus/IBus preedit và bản backport; sau đó chạy lại toàn bộ tests.
3. Tập trung luồng phone-to-PC theo ưu tiên người dùng; tách rõ với server backfill, không báo hoàn thành đồng bộ điện thoại khi chỉ tải lịch sử máy chủ.
4. Kiểm tra tên nhóm và giờ trực tiếp, xử lý các nhóm không có metadata, kiểm tra không tăng unread do lịch sử cũ.
5. Chỉ sau kiểm chứng mới tăng phiên bản, đẩy main và cập nhật bản cài; bảo toàn kho khóa, session và lịch sử mã hóa.

## Tệp hỗ trợ ngoài repository

- Build helper: `/home/triskim/Documents/Codex/2026-11-01/kie/work/build.sh`.
- Diagnostic đọc tài khoản: `/home/triskim/Documents/Codex/2026-11-01/kie/work/diagnostics/check_sync.rs` (đã chuyển khỏi repo vì chứa đường dẫn máy cụ thể).
- Log kiểm tra: `work/live-sync-check.log`, `work/sync-tests.log`, `work/ime-probe.log`.
- Tham khảo giao thức tải từ client công khai: `work/reference/web-shared.js`, `work/reference/web-startup.js`, `work/reference/sync-v2-worker.js`; đây là tài liệu tham khảo, chưa đưa vào sản phẩm.
