#include "Tools/Storage/Image422.hpp"

#include <tmmintrin.h>

// TODO(#1272): add documentation
char Image422::shuffle1[16] = {0, 1, 3, 2, 1, 3, 4, 5, 7, 6, 5, 7, 8, 9, 11, 10};
char Image422::shuffle2[16] = {1, 3, 4, 5, 7, 6, 5, 7, 8, 9, 11, 10, 9, 11, 12, 13};
char Image422::shuffle3[16] = {7, 6, 5, 7, 8, 9, 11, 10, 9, 11, 12, 13, 15, 14, 13, 15};

void Image422::to444Image(Image& image) const
{
  image.resize(get444From422Vector(size));
  unsigned char* src = reinterpret_cast<unsigned char*>(data);
  Color* dst = image.data;
  __m128i shuffle1mm = _mm_loadu_si128(reinterpret_cast<__m128i*>(shuffle1));
  __m128i shuffle2mm = _mm_loadu_si128(reinterpret_cast<__m128i*>(shuffle2));
  __m128i shuffle3mm = _mm_loadu_si128(reinterpret_cast<__m128i*>(shuffle3));

  unsigned char* end = src + sizeof(YCbCr422) * size.x() * size.y();
  for (; src < end; dst += 16, src += 32)
  {
    __m128i yuvpixels1 = _mm_loadu_si128(reinterpret_cast<__m128i*>(src));
    __m128i yuyvpixels1 = _mm_shuffle_epi8(yuvpixels1, shuffle1mm);

    __m128i yuvpixels1point5 = _mm_loadu_si128(reinterpret_cast<__m128i*>(src + 8));
    __m128i yuyvpixels2 = _mm_shuffle_epi8(yuvpixels1point5, shuffle2mm);

    __m128i yuvpixels2 = _mm_loadu_si128(reinterpret_cast<__m128i*>(src + 16));
    __m128i yuyvpixels3 = _mm_shuffle_epi8(yuvpixels2, shuffle3mm);

    _mm_storeu_si128(reinterpret_cast<__m128i*>(dst), yuyvpixels1);
    _mm_storeu_si128(reinterpret_cast<__m128i*>(dst) + 1, yuyvpixels2);
    _mm_storeu_si128(reinterpret_cast<__m128i*>(dst) + 2, yuyvpixels3);
  }
}

Image Image422::to444Image() const
{
  Image image;
  to444Image(image);
  return image;
}
