#include "Framework/Debug/JpegConverter.h"

#include <jpeglib.h>

#define PAD(v, p) ((v + (p)-1) & (~((p)-1)))

class JpegConverter::Impl
{
public:
  Impl();
  ~Impl();

  void convert(const Image& img, CVData& data);
  unsigned long TJBUFSIZE(unsigned long width, unsigned long height);
  void renewBuffer(const Image& img, CVData& data);

private:
  JSAMPROW row_ptr_;
  jpeg_compress_struct cinfo_;
  jpeg_error_mgr jerr_;
  unsigned char* buffer_;
  unsigned long jpegSize_;
  int jpegQuality_;
};


JpegConverter::JpegConverter()
  : pImpl_(new Impl)
{
}

void JpegConverter::convert(const Image& img, CVData& data)
{
  return pImpl_->convert(img, data);
}


JpegConverter::Impl::Impl()
  : jpegSize_(0)
  , jpegQuality_(75)
{
  cinfo_.err = jpeg_std_error(&jerr_);
  jpeg_create_compress(&cinfo_);
  cinfo_.in_color_space = JCS_YCbCr; // JCS_RGB;

  jpeg_set_defaults(&cinfo_);
  jpeg_set_quality(&cinfo_, jpegQuality_, FALSE);

  // Modify after jpeg_set_defaults
  cinfo_.dct_method = JDCT_IFAST; // TODO: Or JDCT_FLOAT need to test performance

  cinfo_.image_width = 640;
  cinfo_.image_height = 480;
}

JpegConverter::Impl::~Impl()
{
  jpeg_destroy_compress(&cinfo_);
}


/*
 * @info From the libjpeg-turbo implementation:
 * @url
 * https://github.com/chris-allan/libjpeg-turbo/blob/a91879e159a3ff3cefd6fdd09093f96355d2cb5f/turbojpeg.c
 */
unsigned long JpegConverter::Impl::TJBUFSIZE(unsigned long width, unsigned long height)
{
  unsigned long retval = 0;
  if (width < 1 || height < 1)
    throw std::runtime_error("Wierd widths and heights incoming");
  // This allows for rare corner cases in which a JPEG image can actually be
  // larger than the uncompressed input (we wouldn't mention it if it hadn't
  // happened before.)
  retval = PAD(width, 16) * PAD(height, 16) * 6 + 2048;
  return retval;
}

void JpegConverter::Impl::renewBuffer(const Image& img, CVData& data)
{
  cinfo_.image_width = img.size.x();
  cinfo_.image_height = img.size.y();
  cinfo_.input_components = 3;

  jpegSize_ = TJBUFSIZE(img.size.x(), img.size.y());
  data.resize(jpegSize_);
  buffer_ = data.data();

  jpeg_mem_dest(&cinfo_, &buffer_, &jpegSize_);
}

void JpegConverter::Impl::convert(const Image& img, CVData& data)
{
  renewBuffer(img, data);

  jpeg_start_compress(&cinfo_, TRUE);

  while (cinfo_.next_scanline < cinfo_.image_height)
  {
    // Maybe room for improvment??
    row_ptr_ = (JSAMPLE*)(&(img.data[cinfo_.next_scanline * cinfo_.image_width]));
    jpeg_write_scanlines(&cinfo_, &row_ptr_, 1);
  }

  jpeg_finish_compress(&cinfo_);
  data.resize(jpegSize_);
}
