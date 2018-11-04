#include "PngConverter.h"

#include <cstdint>
#include <iostream>
#include <png.h>
#include <zlib.h>

using namespace std;

#define ASSERT_EX(cond, error_message)                                                             \
  do                                                                                               \
  {                                                                                                \
    if (!(cond))                                                                                   \
    {                                                                                              \
      std::cerr << error_message;                                                                  \
      exit(1);                                                                                     \
    }                                                                                              \
  } while (0)

struct mem_encode
{
  uint8_t* buffer;
  size_t size;
};

struct TPngDestructor
{
  png_struct* p;
  TPngDestructor(png_struct* p)
    : p(p)
  {
  }
  ~TPngDestructor()
  {
    if (p)
    {
      png_destroy_write_struct(&p, NULL);
    }
  }
};

class PngConverter::Impl
{
public:
  Impl();
  ~Impl();

  void convert(const Image& img, CVData& data);

private:
  void WritePngToMemory(size_t w, size_t h, const uint8_t* dataRGB);
  static void PngWriteCallback(png_structp png_ptr, png_bytep data, png_size_t length);

  png_structp p_;
  png_infop info_ptr_;
  mem_encode menc_;
};

PngConverter::Impl::Impl()
{
  menc_.buffer = nullptr;
  menc_.size = 0;
}

PngConverter::Impl::~Impl()
{
}

void PngConverter::Impl::PngWriteCallback(png_structp png_ptr, png_bytep data, png_size_t length)
{
  mem_encode* p = (mem_encode*)png_get_io_ptr(png_ptr);

  memcpy(p->buffer + p->size, data, length);
  p->size += length;
}

void PngConverter::Impl::WritePngToMemory(size_t w, size_t h, const uint8_t* dataRGB)
{
  p_ = png_create_write_struct(PNG_LIBPNG_VER_STRING, NULL, NULL, NULL);
  ASSERT_EX(p_, "png_create_write_struct() failed");
  info_ptr_ = png_create_info_struct(p_);
  ASSERT_EX(info_ptr_, "png_create_info_struct() failed");
  ASSERT_EX(0 == setjmp(png_jmpbuf(p_)), "setjmp(png_jmpbuf(p) failed");

  png_set_IHDR(p_, info_ptr_, w, h, 8, PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE,
               PNG_COMPRESSION_TYPE_DEFAULT, PNG_FILTER_TYPE_DEFAULT);
  png_set_filter(p_, 0, PNG_FILTER_NONE | PNG_FILTER_VALUE_NONE);
  png_set_compression_level(p_, Z_BEST_SPEED);
  uint8_t** rows = new uint8_t*[h];

  for (size_t y = 0; y < h; ++y)
    rows[y] = const_cast<uint8_t*>(dataRGB) + y * w * 3;

  png_set_rows(p_, info_ptr_, rows);
  png_set_write_fn(p_, &menc_, PngConverter::Impl::PngWriteCallback, NULL);
  png_write_png(p_, info_ptr_, PNG_TRANSFORM_IDENTITY, NULL);

  png_destroy_write_struct(&p_, NULL);

  delete[] rows;
}

void PngConverter::Impl::convert(const Image& img, CVData& data)
{
  data.resize(2 * 1024 * 1024);
  menc_.buffer = data.data();
  menc_.size = 0;
  WritePngToMemory(img.size_.x(), img.size_.y(), (uint8_t*)img.data_);
  data.resize(menc_.size);
}


PngConverter::PngConverter()
  : pImpl_(new Impl)
{
}

void PngConverter::convert(const Image& img, CVData& data)
{
  pImpl_->convert(img, data);
}
