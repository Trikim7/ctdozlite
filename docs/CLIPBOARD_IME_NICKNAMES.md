# Clipboard, Bamboo và biệt danh — 0.1.6

## Dán dữ liệu vào hội thoại

Lỗi Linux cũ nằm ở hai lớp: egui-winit nuốt phím dán nếu clipboard không có UTF-8 text; helper chỉ đọc image/png qua chương trình chưa có trên máy. Nay adapter luôn chuyển hành động Key::Paste, còn GUI chỉ nhận thao tác tại ô soạn tin, đọc dữ liệu ở worker và gắn kết quả với hội thoại ban đầu. Đổi hội thoại/đăng xuất sẽ bỏ tệp chờ, tránh gửi nhầm nơi.

Linux X11 dùng xclip; Wayland ưu tiên wl-paste, có fallback X11 nếu khả dụng. Nhận PNG/JPEG/WebP/BMP, x-special/gnome-copied-files và text/uri-list từ trình quản lý tệp, rồi văn bản. Chỉ URI file cục bộ được coi là tệp, giải mã đúng tên Unicode, bỏ trùng, tối đa 20 tệp. Không tự gửi khi dán; người dùng xem trước, thêm chú thích hoặc hủy rồi bấm Gửi/Enter. Các tệp trong một lô được gửi lần lượt qua send_attachment đã có.

Đọc clipboard có timeout 2 giây mỗi lần và giới hạn 24 MiB; giải mã ảnh có hạn mức kích thước/bộ nhớ. PNG gốc nằm trong tệp tạm riêng (0600 trên Linux), preview tối đa 240px. TempPath được giữ sống qua tác vụ gửi, xóa khi hủy hoặc khi tác vụ kết thúc. Không đưa ảnh clipboard mới vào stickers_cache lâu dài. Multipart encoder cũ dùng RAM nên tệp gửi hiện bị giới hạn 128 MiB, kiểm tra cả trước đọc và khi đọc để ngăn tệp tăng kích thước gây cấp phát không giới hạn.

Khung preview có kích thước giới hạn, tên tệp dài được cắt và có tooltip; ô nhập giữ chỗ cho nút Gửi trong vùng nhìn thấy. Văn bản được chèn theo vị trí/selection của TextEdit với chỉ số ký tự Unicode. Giữ đường đọc ảnh Windows cũ và chuyển ảnh đó qua kho tạm mới; các kiểm tra native đợt này được thực hiện trên Linux X11, không coi Wayland/Windows là đã thử thật.

## Chữ đang gõ qua IBus Bamboo

Đã tái hiện ngoại tuyến trên winit 0.30.13: từ đầu tiên có Preedit; sau Space, XLookupString reset is_composing, trong khi Bamboo tái sử dụng preedit session và gửi Update cho từ sau mà không gửi Start mới. Guard `if self.is_composing` bỏ mất các cập nhật này, nên chỉ thấy từ sau Space/Enter.

Vendor winit giữ nguyên version/API/dependencies, chỉ nhận Update hợp lệ và bật composing lại. Vendor egui-winit vẫn xử lý Commit và ImeState ngăn Enter chốt dấu bị hiểu là gửi. Không đổi cấu hình gõ toàn hệ thống, không giả lập sửa dấu bằng backspace.

Probe native với cả Bamboo và Bamboo::Us cho thấy `tiếng`, `tiếng việt`, `tiếng việt đang` xuất hiện trước từng Space. Enter chốt `chào` không tăng số gửi giả, Enter tiếp theo tăng đúng một. Probe không kết nối Zalo. Engine ban đầu được khôi phục sau thử.

## Biệt danh

Đối chiếu mã bundle Web đã có: GET /api/alias/list/v2 (fallback /api/alias/list), tham số page/count/imei/src, items chứa userId và alias, hasMore cho phân trang. Không dùng tên hiển thị thông thường thay cho alias. Service alias được nhận từ cấu hình máy chủ, có default tương thích phiên cũ.

GUI giữ alias map riêng: chỉ hiển thị/tìm kiếm theo biệt danh ở sidebar, tiêu đề và thông tin hội thoại; Thread.name và Thread.id không đổi. Hai người trùng biệt danh không bị gộp. Làm mới khi mở app, theo chu kỳ 5 phút hoặc khi bấm đồng bộ danh sách. Alias trống/xóa được bỏ khi thay snapshot mới. Cache biệt danh mã hóa AES-GCM riêng bằng kho khóa hệ điều hành, ràng buộc owner, ghi nguyên tử và không đè cache không giải mã được.

Đã đọc tài khoản thật: 35 alias, 32 khớp với các hội thoại hiện có, cache mã hóa đọc lại đúng. Không in tên, ID, token hoặc dữ liệu phản hồi vào log chẩn đoán; không cập nhật biệt danh lên máy chủ.

## Kiểm chứng

139 test thư viện đạt, 4 ignore mặc định; release Linux build đạt. Có test URI Unicode và từ chối remote, giữ kích thước ảnh/preview và xóa tệp tạm, chèn văn bản thay selection Unicode, phân trang alias/error và tách biệt canonical identity. Đã thử native Bamboo và GUI Ctrl+V ảnh, Shift+Insert nhiều tệp, văn bản tiếng Việt; clipboard gốc được khôi phục theo định dạng chính. Các phép thử không gửi tin/tệp lên Zalo; khâu upload giữ đường backend sẵn có, chưa xác minh giao nhận tệp thật trong đợt này.

Không thêm dependency Rust: winit là dependency đã có, được vendor để vá X11. xclip là công cụ nhỏ cài ở thư mục bin của người dùng trên máy thử; launcher đưa thư mục này vào PATH. Giữ các giới hạn texture/ảnh/writer từ 0.1.4–0.1.5. Chưa coi các vấn đề đồng bộ Android toàn bộ hoặc URL media từ phone import đã được giải quyết bằng đợt này.
