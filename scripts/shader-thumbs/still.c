/* Minimal libretro core for the shader thumbnails: shows one still picture forever.
 * Content = raw XRGB8888 pixels, 320 pixels wide (height = size / 1280), as written by
 * render.sh from testimage.py. Build: cc -O2 -shared -fPIC -I<dir of libretro.h> still.c -o still_libretro.so */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "libretro.h"

#define WIDTH 320

static uint32_t *pixels;
static unsigned height;
static retro_video_refresh_t video_cb;
static retro_environment_t env_cb;

RETRO_API void retro_set_environment(retro_environment_t cb) {
  bool no_game = false;
  env_cb = cb;
  cb(RETRO_ENVIRONMENT_SET_SUPPORT_NO_GAME, &no_game);
}
RETRO_API void retro_set_video_refresh(retro_video_refresh_t cb) { video_cb = cb; }
RETRO_API void retro_set_audio_sample(retro_audio_sample_t cb) { (void)cb; }
RETRO_API void retro_set_audio_sample_batch(retro_audio_sample_batch_t cb) { (void)cb; }
RETRO_API void retro_set_input_poll(retro_input_poll_t cb) { (void)cb; }
RETRO_API void retro_set_input_state(retro_input_state_t cb) { (void)cb; }
RETRO_API void retro_init(void) {}
RETRO_API void retro_deinit(void) { free(pixels); pixels = NULL; }
RETRO_API unsigned retro_api_version(void) { return RETRO_API_VERSION; }

RETRO_API void retro_get_system_info(struct retro_system_info *info) {
  memset(info, 0, sizeof(*info));
  info->library_name = "still";
  info->library_version = "1";
  info->valid_extensions = "rgb";
  info->need_fullpath = true;
}

RETRO_API void retro_get_system_av_info(struct retro_system_av_info *info) {
  memset(info, 0, sizeof(*info));
  info->geometry.base_width = info->geometry.max_width = WIDTH;
  info->geometry.base_height = info->geometry.max_height = height;
  info->geometry.aspect_ratio = 4.0f / 3.0f;
  info->timing.fps = 60.0;
  info->timing.sample_rate = 48000.0;
}

RETRO_API bool retro_load_game(const struct retro_game_info *game) {
  enum retro_pixel_format fmt = RETRO_PIXEL_FORMAT_XRGB8888;
  FILE *f;
  long size;
  if (!game || !game->path || !env_cb(RETRO_ENVIRONMENT_SET_PIXEL_FORMAT, &fmt))
    return false;
  if (!(f = fopen(game->path, "rb")))
    return false;
  fseek(f, 0, SEEK_END);
  size = ftell(f);
  rewind(f);
  height = (unsigned)(size / (WIDTH * 4));
  pixels = malloc((size_t)size);
  if (!height || !pixels || fread(pixels, 1, (size_t)size, f) != (size_t)size) {
    fclose(f);
    return false;
  }
  fclose(f);
  return true;
}

RETRO_API void retro_run(void) { video_cb(pixels, WIDTH, height, WIDTH * 4); }
RETRO_API void retro_unload_game(void) {}
RETRO_API void retro_reset(void) {}
RETRO_API void retro_set_controller_port_device(unsigned port, unsigned device) { (void)port; (void)device; }
RETRO_API unsigned retro_get_region(void) { return RETRO_REGION_NTSC; }
RETRO_API size_t retro_serialize_size(void) { return 0; }
RETRO_API bool retro_serialize(void *data, size_t size) { (void)data; (void)size; return false; }
RETRO_API bool retro_unserialize(const void *data, size_t size) { (void)data; (void)size; return false; }
RETRO_API void retro_cheat_reset(void) {}
RETRO_API void retro_cheat_set(unsigned index, bool enabled, const char *code) { (void)index; (void)enabled; (void)code; }
RETRO_API bool retro_load_game_special(unsigned type, const struct retro_game_info *info, size_t num) { (void)type; (void)info; (void)num; return false; }
RETRO_API void *retro_get_memory_data(unsigned id) { (void)id; return NULL; }
RETRO_API size_t retro_get_memory_size(unsigned id) { (void)id; return 0; }
