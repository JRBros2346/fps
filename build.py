from PIL import Image
import os

file = open("header.h", "w")


def map_texture():
    img = Image.open("textures/map.bmp")
    hi = 0
    lo = 0
    win = []
    for y in range(img.height):
        for x in range(img.width):
            pixel = img.getpixel((x, y))
            bit = pixel[0] == 0 and pixel[1] == 0 and pixel[2] == 0
            if y < 8:
                hi = (hi << 1) | bit
            else:
                lo = (lo << 1) | bit
            if pixel[0] == 128 and pixel[1] == 128 and pixel[2] == 128:
                win.append((x, y))
    hi_hi = (hi >> 64) & 0xFFFFFFFFFFFFFFFF
    hi_lo = hi & 0xFFFFFFFFFFFFFFFF
    lo_hi = (lo >> 64) & 0xFFFFFFFFFFFFFFFF
    lo_lo = lo & 0xFFFFFFFFFFFFFFFF
    file.write(
        "static const unsigned __int128 MAP[2] = {{\n"
        "    ((unsigned __int128)0x{:016x}ULL << 64 | 0x{:016x}ULL),\n"
        "    ((unsigned __int128)0x{:016x}ULL << 64 | 0x{:016x}ULL),\n"
        "}};\n".format(hi_hi, hi_lo, lo_hi, lo_lo)
    )
    file.write("static const unsigned WIN[][2] = {\n")
    for x, y in win:
        file.write(f"    {{{x}, {y}}},\n")
    file.write("};\n")


def wall_texture():
    img = Image.open("textures/wall.bmp")
    content = [0 for _ in range(16)]
    for y in range(img.height):
        for x in range(img.width):
            pixel = img.getpixel((x, y))
            code = int((pixel[0] + pixel[1] + pixel[2]) / 3 * 23 / 255) + 232
            content[y] |= (code & 0xFF) << (8 * (15 - x))
    file.write("static const unsigned __int128 WALL[16] = {\n")
    for row in content:
        hi = (row >> 64) & 0xFFFFFFFFFFFFFFFF
        lo = row & 0xFFFFFFFFFFFFFFFF
        file.write(f"    ((unsigned __int128)0x{hi:016x}ULL << 64 | 0x{lo:016x}ULL),\n")
    file.write("};\n")


def died():
    with open("textures/died.ascii", "r") as fin:
        lines = [line.rstrip("\n") for line in fin]
    width = max(len(line) for line in lines)
    file.write(f"static const char* DIED[{len(lines)}] = {{\n")
    for line in lines:
        file.write(f'    "{line.replace("\\", "\\\\")}",\n')
    file.write("};\n")
    file.write(f"static const int DIED_WIDTH = {width};\n")
    file.write(f"static const int DIED_HEIGHT = {len(lines)};\n")


def sixel():
    header = "\\x1bPq"
    for r in range(6):
        for g in range(6):
            for b in range(6):
                code = 16 + r * 36 + g * 6 + b
                rr = 100 * (r * 8 + 11 if r != 0 else 0) // 51
                gg = 100 * (g * 8 + 11 if g != 0 else 0) // 51
                bb = 100 * (b * 8 + 11 if b != 0 else 0) // 51
                header += f"#{code};2;{rr};{gg};{bb}"
    for gray in range(24):
        l = gray * 10 + 8
        code = 232 + gray
        header += f"#{code};2;{l};{l};{l}"
    file.write(f'#define SIXEL_HEADER "{header}"\n')
    file.write('#define SIXEL_FOOTER "\\x1b\\\\"\n')


map_texture()
wall_texture()
died()
sixel()


file.close()

file = open("flag.h", "w")


def flag():
    flag = b"expX{FLAG}"
    encoded = [b ^ 0x42 for b in flag]
    file.write(f"char FLAG[{len(encoded)}] = {{")
    file.write(", ".join(str(b) for b in encoded))
    file.write("};\n")
    file.write(f"size_t FLAG_SIZE = {len(flag)};\n")


flag()
file.close()

os.system("clang main.c -o xeno -lm")
