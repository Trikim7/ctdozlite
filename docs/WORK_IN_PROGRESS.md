# Tiến độ ctdozlite — 01/11/2026

## Yêu cầu

Tự tải tin nhắn cũ, ưu tiên đồng bộ trực tiếp từ điện thoại; hỗ trợ IBus Bamboo; lấy tên nhóm thật; hiển thị giờ GMT+7.

## Đã xác minh

- 215/215 nhóm lấy được tên, 79 avatar. Chẩn đoán chỉ in số lượng/trường dữ liệu, không in tên, ID, nội dung hoặc khóa.
- GMT+7 cho tin nhắn, hội thoại, giờ đọc SQLite và cache cũ. Xóa giờ giả của hội thoại không có tin; metadata mới có thể sửa giờ cũ.
- IBus Bamboo/X11 đã gõ đúng “tiếng việt” trong native probe ngoại tuyến. Enter đầu commit không gửi, Enter sau mới kích hoạt gửi giả. Không gửi tin thử vào tài khoản thật, không đổi cấu hình Bamboo toàn hệ thống.
- Vendor egui-winit có giấy phép và ghi nguồn upstream. Bổ sung xử lý XIM Disabled đến trước Commit và Commit không có Preedit để không mất từ thứ hai.
- Backfill nhóm/Cloud tự chạy một lần sau đăng nhập: phân trang đến hết, chặn cursor lặp, lưu từng trang mã hóa thay vì ghi lại cả hội thoại mỗi trang, giữ timestamp mới nhất trong SQLite, không báo unread. Có dừng/thử lại.
- Kiểm tra trực tiếp nhận được 25 tin ở trang gần nhất của một nhóm; yêu cầu trang cũ trả lỗi -69. Chưa xác nhận tải hết nhiều trang trên tài khoản thật; app báo thất bại thay vì coi đó là lịch sử trống.
- Bộ thử thư viện cuối: 118 pass, 0 fail, 3 ignore, gồm sửa SQLite. Release build thành công. Clippy với -D warnings chưa đạt vì 42 cảnh báo ở mã cũ; không coi lint là đạt.

## Đồng bộ điện thoại — còn dang dở

- Chưa gửi yêu cầu chuyển lịch sử thật vì receiver/importer chưa hoàn chỉnh.
- Đã xác minh ngoại tuyến bằng dữ liệu giả: crypto WASM chính thức tạo identity/ephemeral, mã hóa/giải mã metadata và message, kiểm tra chống sửa ciphertext.
- Đã thử trọn định dạng: protobuf -> Zstd -> mã hóa -> file chunk có u32 little-endian -> giải mã -> giải nén -> protobuf. Giữ đúng tiếng Việt và ID lớn. Đây không phải bằng chứng đồng bộ với điện thoại thật.
- Thành phần proprietary tải công khai chỉ nằm ngoài repo trong work/reference, không phân phối cùng app. Binary Rust không phụ thuộc Node.js.
- Còn cần runtime crypto được phép phân phối hoặc backend tương thích; nối request/receiver trên socket hiện có; quản lý xác nhận/cancel trên điện thoại; ánh xạ globalNoisedId với UID hội thoại; nhập dữ liệu thật vào kho mã hóa và thử trên thiết bị.
- Không đánh dấu yêu cầu đồng bộ điện thoại hoàn tất chỉ vì tải được lịch sử máy chủ.

## Điểm tiếp tục và dữ liệu

- Nhánh: wip/history-ibus-groups-vntime; commit lưu trước lần tiếp tục: 9aebef1.
- Mã nguồn: work/ctdozlite; build helper: work/build.sh (đường dẫn máy cụ thể ngoài repo).
- Dữ liệu thật: ~/.local/share/ctdozlite. Khóa gắn với đường dẫn tuyệt đối; giữ nguyên dữ liệu và kho khóa khi cài cập nhật.
- Chẩn đoán đọc tài khoản: work/diagnostics/check_sync.rs, đã chuyển ra ngoài repo.
- IME probe: src/bin/ime_probe.rs; script thao tác ngoại tuyến work/type_ime_probe.py.
- Nghiên cứu: work/reference/full-transfer-probe.cjs, crypto-probe.cjs, protobuf-probe.cjs và client JS/WASM công khai. Không đưa dữ liệu/khóa thật vào các probe này.
- Nhật ký: work/final-012-tests.log, work/final-012-clippy.log, work/release-012.log, work/live-pagination*.log.


## Cập nhật 0.1.3

Người dùng xác nhận dùng Android, sẵn sàng và đã bấm cho phép đồng bộ trên điện thoại. Đã triển khai adapter Node 24 + runtime Zalo tải riêng, pin SHA256; receiver dùng cùng socket GUI, khóa chỉ trong bộ nhớ, nhập từng lô vào lịch sử mã hóa. Đã nhận và lưu 6.148 tin thật. Phiên CLI dừng ở lỗi SQLite database is locked, khi đồng thời xuất hiện hai cửa sổ bản cũ. Chưa coi toàn bộ lịch sử đã đồng bộ xong.

Đã sửa nhận control data dạng JSON string, ánh xạ /api/gid/encrypt và decrypt có hai lớp data, chuyển enum loại tin của điện thoại sang loại tin GUI. Thêm giao dịch nguyên tử SQLite mỗi đợt lưu và chặn mở nhiều GUI. Bổ sung --sync-phone để ưu tiên đồng bộ Android một lần sau đăng nhập; nút đồng bộ Android và dừng trên banner.

Nút X nay thoát, lưu cache mã hóa khi thoát. Ảnh nhận vào có nhãn loại media đúng; hỗ trợ thumbnail WebP/GIF. Video phát trong hội thoại bằng FFmpeg và FFplay; tệp tải riêng 0600, tối đa 256 MB, giải mã chỉ dùng file/pipe, có play/pause/replay, dừng khi chuyển hội thoại. Đã thử thành công phát/pause/resume và xóa file tạm bằng fixture ngoại tuyến. 123 bài test thư viện đạt, 4 ignored; riêng test playback ignored đã được chạy và đạt trên máy này.

Log chỉ số: work/phone-real-sync2.log; không in nội dung, ID, khóa. Diagnostic CLI src/bin/phone_sync.rs (--diagnose, --self-test, --run); --run chỉ dùng khi GUI đã đóng. Không mở hai kết nối đồng bộ song song. Tiếp tục thử bằng bản mới để tránh tranh chấp với cửa sổ cũ. FFmpeg/FFplay được cài riêng trong ~/.local/lib/ctdozlite-media và wrapper ~/.local/bin, không đổi gói hệ thống.


## Cập nhật 0.1.4 — xử lý tăng RAM

Đã rà soát việc render toàn bộ lịch sử khiến tải ảnh ngoài viewport, texture cache không giới hạn, nhiều tác vụ spawn_blocking cùng giữ snapshot toàn bộ hội thoại, và Mapping đồng bộ giữ bản sao lịch sử không cần thiết. Thay bằng tải ảnh theo viewport, LRU texture tối đa 64 MiB/256 ảnh, ba permit giữ qua cả hàng đợi GUI, một writer + một snapshot chờ được thay bằng bản mới nhất, và metadata-only Mapping. Cache cũ vẫn được kiểm tra xác thực và JSON trước khi ghi, nhưng không deserialize lại toàn bộ Thread. Khung hình cuộc gọi chờ GUI giới hạn hai, rớt khung khi bận.

128 bài kiểm thử thư viện đạt; gồm hội thoại giả 5.000 ảnh chỉ tạo yêu cầu gần viewport, loại texture/giải phóng handle, coalescing snapshot và giới hạn hàng đợi ảnh. Test native_history_and_cache_roundtrip_and_missing_key_protection được chạy riêng và đạt, dùng khóa tạm rồi xóa, không thay khóa người dùng. Release build Linux thành công. Các con số này chưa chứng minh ổn định RAM trong một phiên đồng bộ Android đầy đủ. Tình trạng URL media sau import và hoàn tất toàn bộ đồng bộ từ checkpoint trước vẫn cần kiểm chứng riêng.

## Cập nhật 0.1.5 — settings, avatar, thiết bị gọi

Người dùng yêu cầu tiếp tục dùng REA đối chiếu UI gốc, sửa settings, chuẩn bị cuộc gọi và avatar, giữ gọn nhẹ. Đã dùng REA trên các module Web chọn lọc, ghi rõ giới hạn trong UI_AND_CALLS.md. Sửa geometry/scroll/compact và overflow thẻ preview; sửa avatar crop và retry; kiểm tra thành phần gọi theo yêu cầu, thêm chọn loa, cache danh sách thiết bị. Thành phần Opus/FFmpeg/H.264/ALSA và camera đã có và kiểm tra khả năng tại máy đạt, chưa thực hiện cuộc gọi đầu-cuối hoặc ghi âm/quay camera. Không thêm dependencies.

133 test thư viện đạt, 4 ignore; release Linux đạt; fixture H.264 và Opus roundtrip đạt. Giao diện thật đã kiểm tra 1020×700 và 480×360. Giữ các giới hạn bộ nhớ của 0.1.4, bỏ cache load/merge và SQLite hydrate lặp lúc khởi động. Các việc đồng bộ Android hoàn tất, media URL sau import và lịch sử SQLite cũ từ checkpoint trước vẫn chưa được coi đã giải quyết trong đợt sửa UI này.

## Cập nhật 0.1.6 — dán clipboard, preedit Bamboo, alias

Đã tái hiện và sửa winit X11 bỏ Update sau commit của Bamboo; probe native với Bamboo và Bamboo::Us hiện cả từ 2/3 trước Space, Enter chốt dấu không gửi. Giữ nguyên cấu hình/engine gốc sau thử. Dán ảnh/tệp/văn bản đọc qua worker, có preview/hủy, lifetime tệp tạm riêng, ID hội thoại để bỏ khi chuyển nơi, giới hạn clipboard/upload nhằm giữ RAM. xclip được cài user-local. Đã thử native Ctrl+V ảnh, Shift+Insert hai tệp tên Unicode và văn bản; không gửi tin thử, khôi phục clipboard theo định dạng chính. Đã sửa thêm preview quá rộng đẩy nút Gửi ra ngoài viewport.

Alias API đọc được 35 mục, 32 khớp hội thoại; cache mã hóa roundtrip đạt. Hiển thị theo map riêng thay vì ghi đè canonical Thread.name/ID, tự làm mới. 139 test thư viện đạt, 4 ignore; release Linux build đạt. Xem CLIPBOARD_IME_NICKNAMES.md cho giới hạn và bằng chứng; các việc Android/media URL trước đó vẫn chưa hoàn tất.
