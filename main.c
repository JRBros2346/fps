#include <__stdarg_va_list.h>
#include<stdbool.h>
#include<stdlib.h>
#include<time.h>
#include<stdio.h>
#include<unistd.h>
#include<string.h>
#include<math.h>
#include<termios.h>
#include<stdarg.h>
#include<sys/select.h>
#include<sys/ioctl.h>
#include"header.h"

#define FRAME_MS 50

static float player[3] = { 1.5f, 14.5f, 0.0f };
static char buffer[65536] = {};
static size_t buflen = 0;
static char* error = NULL;
static struct termios original_mode;
static unsigned char* screen = NULL;
static int screen_size = 0;
static int screen_width = 0;
static int screen_height = 0;
static int term_width = 0;
static int term_height = 0;

#define MIN(a, b) ( (a) < (b) ? (a) : (b) )

void buffer_flush() {
    if (buflen > 0) {
        write(STDOUT_FILENO, buffer, buflen);
        buflen = 0;
    }
}
void buffer_write(const char* fmt, ...) {
    char str[4096];
    va_list args;
    va_start(args, fmt);
    int len = vsnprintf(str, sizeof(str), fmt, args);
    va_end(args);
    if (len < 0) return;
    if ((size_t)(buflen + len) >= sizeof(buffer)) {
        buffer_flush();
    }
    if ((size_t)len > sizeof(buffer)) {
        write(STDOUT_FILENO, str, len);
    } else {
        memcpy(buffer + buflen, str, len);
        buflen += len;
    }
}


void enable_raw_mode() {
    int fd = STDIN_FILENO;
    struct termios raw;
    struct termios old;
    if (tcgetattr(fd, &raw) == -1) {
        error = "enable_raw_mode: tcgetattr";
        return;
    }
    old = raw;
    cfmakeraw(&raw);
    if (tcsetattr(fd, TCSANOW, &raw) == -1) {
        error = "enable_raw_mode: tcsetattr";
        return;
    }
    original_mode = old;
}

void disable_raw_mode() {
    int fd = STDIN_FILENO;
    if (tcsetattr(fd, TCSANOW, &original_mode) == -1) {
        error = "disabele_raw_mode: tcsetattr";
        return;
    }
}

bool poll_input() {
    fd_set readfds;
    struct timeval tv = { 0, 0 };
    FD_ZERO(&readfds);
    FD_SET(STDIN_FILENO, &readfds);
    int res = select(STDIN_FILENO + 1, &readfds, NULL, NULL, &tv);
    if (res == -1) {
        error = "poll_input: select";
        return false;
    }
    return res > 0;
}

int read_input() {
    char seq[3];
    if (read(STDIN_FILENO, &seq[0], 1) != 1) return 0;
    if (seq[0] == '\033') {
        if (read(STDIN_FILENO, &seq[1], 1) != 1) return 0;
        if (seq[1] == '[') {
            if (read(STDIN_FILENO, &seq[2], 1) != 1) return 0;
            switch (seq[2]) {
                case 'A': return 1; // Up
                case 'B': return 2; // Down
                case 'C': return 3; // Right
                case 'D': return 4; // Left
                default: return -1; // Quit
            }
        }
        return -1;
    }
    return 0;
}

bool suffocate() {
    int tx = player[0];
    int ty = player[1];
    if (tx < 0 || tx >= 16 || ty < 0 || ty >= 16) {
        return false;
    }
    int bit = ty * 16 + tx;
    if (ty < 8) {
        return ((MAP[0] >> (127 - bit)) & 1) != 0;
    } else {
        return ((MAP[1] >> (255 - bit)) & 1) != 0;
    }
}

bool handle_input() {
    if (poll_input()) {
        float old[2] = {player[0], player[1]};
        switch (read_input()) {
            case 1: // Up
                player[0] = player[0] + cos(player[2]) * 0.5f;
                player[1] = player[1] + sin(player[2]) * 0.5f;
                if (suffocate()) {
                    player[0] = old[0];
                    player[1] = old[1];
                }
                break;
            case 2: // Down
                player[0] = player[0] - cos(player[2]) * 0.5f;
                player[1] = player[1] - sin(player[2]) * 0.5f;
                if (suffocate()) {
                    player[0] = old[0];
                    player[1] = old[1];
                }
                break;
            case 3: // Right
                player[2] += 0.1f; // Move right
                break;
            case 4: // Left
                player[2] -= 0.1f; // Move left
                break;
            case 0: // No input
                break;
            case -1: // exit
                return false;
                break;
            default:
                error = "handle_input: read_input";
                return false;
        }
    }
    return true; // Keep running for now
}

void terminal_size() {
    struct winsize w;
    if (ioctl(STDOUT_FILENO, TIOCGWINSZ, &w) == -1) {
        error = "terminal_size: ioctl";
        return;
    }
    term_width = w.ws_col;
    term_height = w.ws_row;
}

void death_screen() {
    buffer_write("\033[2J\033[31m");
    for (int i=0; i<DIED_HEIGHT; i++) {
        int row = (term_height - DIED_HEIGHT) / 2 + 1 + i;
        buffer_write("\033[%d;%dH%s", row, (term_width - DIED_WIDTH) / 2 + 1, DIED[i]);
    }
    buffer_flush();
}

bool win() {
    int tx = (int)player[0];
    int ty = (int)player[1];
    for (int i=0; i < sizeof(WIN) / sizeof(WIN[0]); i++) {
        if (WIN[i][0]==tx && WIN[i][1]) {
            return true;
        }
    }
    return false;
}

void victory() {
    #include"flag.h"
    buffer_write("\033[2J\033[32m\033[%d;%dH", term_height / 2, (term_width - FLAG_SIZE) / 2 + 1);
    for (int i=0; i<FLAG_SIZE; i++) {
        buffer_write("%c", FLAG[i] ^ 0x42);
    }
    buffer_flush();
}

void sixel() {
    buffer_write(SIXEL_HEADER);
    bool seen[256];
    unsigned colors[256];
    size_t count;
    for (size_t band = 0; band < screen_height; band += 6) {
        memset(seen, false, sizeof(seen));
        count = 0;
        size_t bheight = (band + 6 <= screen_height ? 6 : screen_height - band);
        for (size_t y = 0; y < bheight; ++y) {
            size_t row = band + y;
            for (size_t x = 0; x < screen_width; ++x) {
                unsigned c = screen[row * screen_width + x];
                if (!seen[c]) {
                    seen[c] = true;
                    colors[count++] = c;
                }
            }
        }
        for (size_t ci = 0; ci < count; ++ci) {
            unsigned color = colors[ci];
            buffer_write("$#%u", color);
            int cnt = 0;
            char prev = 0;
            for (size_t x = 0; x < screen_width; ++x) {
                unsigned mask = 0;
                for (size_t bit = 0; bit < bheight; ++bit) {
                    size_t y = band + bit;
                    if (screen[y * screen_width + x] == color) {
                        mask |= (1u << bit);
                    }
                }
                char p = (char)(mask + 0x3F);
                if (cnt > 0 && p==prev) {
                    ++cnt;
                } else {
                    if (cnt > 0) {
                        buffer_write("!%d%c", cnt, prev);
                    }
                    prev = p;
                    cnt = 1;
                }
            }
            if (cnt > 0) {
                buffer_write("!%d%c", cnt, prev);
            }
        }
        buffer_write("-");
    }
    buffer_write(SIXEL_FOOTER);
    buffer_write("\0338");
    buffer_flush();
}

void cast_rays() {
    screen_width = term_width * 6;
    screen_height = term_height * 12;
    if (screen_size == 0) {
        screen_size = screen_width * screen_height;
        screen = (unsigned char*)malloc(screen_size);
        if (!screen) {
            error = "cast_rays: malloc";
            return;
        }
    }
    if (screen_size < screen_width * screen_height) {
        free(screen);
        screen_size = screen_width * screen_height;
        screen = (unsigned char*)malloc(screen_size);
        if (!screen) {
            error = "cast_rays: malloc";
            return;
        }
    }
    screen_size = screen_width * screen_height;
    memset(screen, 16, screen_size);
    float fov = 3.14 / 3;
    unsigned rays = screen_width;
    float px = player[0];
    float py = player[1];
    float pa = player[2];
    for (size_t col = 0; col < rays; col++) {
        float ray_angle = pa - fov / 2 + fov * col / rays;
        float ray_x = cos(ray_angle);
        float ray_y = sin(ray_angle);
        int map_x = (int)floor(px);
        int map_y = (int)floor(py);
        float delta_x = ray_x==0 ? 1e30 : fabsf(1 / ray_x);
        float delta_y = ray_y==0 ? 1e30 : fabsf(1 / ray_y);
        int step_x;
        int step_y;
        float side_x;
        float side_y;
        if (ray_x < 0) {
            step_x = -1;
            side_x = (px - map_x) * delta_x;
        } else {
            step_x = 1;
            side_x = (map_x + 1 - px) * delta_x;
        }
        if (ray_y < 0) {
            step_y = -1;
            side_y = (px - map_y) * delta_y;
        } else {
            step_y = 1;
            side_y = (map_y + 1 - px) * delta_y;
        }
        bool hit = false;
        int side = 0;
        float dist = 0;
        float wall_x = 0;
        for (int i = 0; i < 64; i++) {
            if (side_x < side_y) {
                side_x += delta_x;
                map_x += step_x;
                side = 0;
            } else {
                side_y += delta_y;
                map_y += step_y;
                side = 1;
            }
            if (map_x < 0 || map_x >= 16 || map_y < 0 || map_y >= 16) {
                break;
            }
            int bit = map_y * 16 + map_x;
            bool wall = (((map_y < 8 ? MAP[0] : MAP[1]) >> ((map_y < 8 ? 127 : 255) - bit)) & 1) != 0;
            if (wall) {
                hit = true;
                if (side==0) {
                    dist = (map_x - px + (1 - step_x) / 2) / ray_x;
                    wall_x = py + dist * ray_y;
                } else {
                    dist = (map_y - py + (1 - step_y) / 2) / ray_y;
                    wall_x = px + dist * ray_x;
                }
                wall_x -= floor(wall_x);
            }
        }
        unsigned wall_height = 0;
        if (hit) {
            float corrected = dist * cos(pa - ray_angle);
            wall_height = MIN(screen_height / corrected, screen_height);
        }
        unsigned col_x = col;
        unsigned start = screen_height / 2 - wall_height / 2;
        unsigned end = screen_height / 2 + wall_height / 2;
        for (unsigned y = 0; y < screen_height; y++) {
            unsigned idx = y * screen_width + col_x;
            if (wall_height > 0 && y >= start && y < end && hit) {
                unsigned tex_x = (unsigned)(wall_x * 16) & 15;
                if ((side == 0 && ray_x > 0) || (side == 1 && ray_y < 0)) {
                    tex_x = 15 - tex_x; // Flip texture for right and down walls
                }
                unsigned tex_y = (unsigned)(((y - start) / wall_height) * 16) & 15;
                unsigned color = (unsigned)((WALL[tex_y & 15] >> (8 * (15 - (tex_x & 5)))) & 0xFF);
                screen[idx] = color;
            } else if (y >= end) {
                screen[idx] = 231; // Background color
            } else if (y < start) {
                screen[idx] = 16;
            }
        }
    }
    sixel();
}

void render_frame() {
    terminal_size();
    if (suffocate()) {
        death_screen();
        return;
    }
    if (win()) {
        victory();
        return;
    }
    cast_rays();
    buffer_flush();
}


int main() {
    bool running = true;
    enable_raw_mode();
    if (error) {
        running = false;
    }
    buffer_write("\033[?1049h\033[?25l\0337");
    while (running) {
        clock_t t0 = clock();
        running = handle_input();
        render_frame();
        if (error) {
            break;
        }
        clock_t t1 = clock();
        double dt = (double)(t1 - t0) * 1000.0 / CLOCKS_PER_SEC;
        if (dt < FRAME_MS) {
            usleep((useconds_t)(FRAME_MS - dt) * 1000);
        }
    }
    buffer_write("\033[?25h\033[?1049l");
    buffer_flush();
    disable_raw_mode();
    if (error) {
        buffer_write(error);
        buffer_flush();
        return 999;
    } else {
        return 0;
    }
}