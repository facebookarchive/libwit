SRC = src/vad/vad.c
OBJ = $(SRC:.c=.o)
OUT = libvad.a
CFLAGS = -std=c99
 
default: $(OUT)
 
$(OUT): $(OBJ)
	ar rcs $(OUT) $(OBJ)
 
clean:
	rm -f $(OBJ) $(OUT) Makefile.bak
