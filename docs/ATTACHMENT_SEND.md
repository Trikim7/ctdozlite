# Gửi tệp 0.1.7

Zalo có hai giai đoạn: HTTP upload trả `fileId`, sau đó control `file_done` trả URL. Backend cũ gửi tiếp ngay khi có ID, bỏ qua URL bất đồng bộ và chỉ ghi lỗi vào status tooltip. Ba cách nhập tệp dùng chung backend nên cùng bị ảnh hưởng.

- Đăng ký nhận completion trước upload, tránh sự kiện đến trước HTTP response. Nhận qua socket 601/603 hoặc control polling. Ghép chính xác owner + fileId; hàng đợi broadcast tối đa 32 sự kiện, không lưu lịch sử control.
- Chỉ dùng completion file_done/video_done, URL HTTPS thuộc CDN đã được kiểm tra. JSON content/data dạng chuỗi có giới hạn. Chờ tối đa 90 giây, quá hạn báo lỗi và không gửi tin thiếu URL.
- Kiểm tra mã lỗi từng chunk, mã tệp, URL và mã tin nhắn thật. Bỏ fallback gửi lại cả tệp sau lỗi ở bước submit, tránh che lỗi/nhân đôi tin. Không tạo tin thành công giả với URL local.
- Các đường paste, drop, picker chia sẻ một permit; giữ tối đa một bộ tệp trong RAM và giải phóng bytes trước khi chờ completion. Giới hạn 128 MiB/tệp vẫn giữ nguyên.
- Lỗi gửi hiện trong hội thoại đến khi bấm Đóng; thêm thông báo đang tải lên/gửi xong. Không chỉ giấu trong tooltip.

## Xác minh

144 test thư viện đạt, 4 ignore mặc định. Bao gồm fixture phản hồi upload thành công nhưng chưa có URL, control đến sớm, sai owner/fileId, URL không tin cậy, timeout và phản hồi không có xác nhận tin nhắn.

Sau khi người dùng cho phép, đã gửi tệp văn bản nhỏ vào đúng send2me_id (Cloud của tôi). HTTP upload chỉ có chunkId/clientFileId/fileId/finished; sau khi xử lý completion, submit trả error_code 0, msgId thật và URL HTTPS. Probe chỉ in các boolean/mã kết quả, không in ID/URL/cookie. Không gửi cho người khác. Bản cài 0.1.7 hiển thị tệp Cloud từ lịch sử máy chủ với trạng thái Đã nhận.

Đã thử native: nút chọn tệp mở zenity; Ctrl+V URI tệp hiện preview đúng tên; Esc hủy; khôi phục clipboard. Phần thử GUI này không bấm Gửi. Kéo thả dùng cùng backend đã sửa nhưng chưa mô phỏng lại sự kiện kéo thả native trong đợt này.

Nguồn: module DLcZ/fBUP/UYGI trong bản web-shared.js đã lưu (SHA256 f1f574c135e0016d0896910d12b7886cf65cb6b791bd9e813d4d7642e662fcba); đối chiếu uploadAttachment/listen trong mã zca-js. REA static evidence ev_34e43c1f786f1eb5fe06a680f7b38e82a69ca570dd23b285d69736ef927eda29. Phân tích REA chỉ tĩnh, có giới hạn semantic; kết quả gửi thật được kiểm tra riêng.
