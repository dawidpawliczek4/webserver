TARGET = webserver

.PHONY: all clean distclean

all: $(TARGET)

$(TARGET):
	cargo build --release
	cp target/release/$(TARGET) ./$(TARGET)

clean:
	cargo clean

distclean: clean
	rm -f $(TARGET)
