#include "Modules/Debug/JpegConverter.h"

#include <jpeglib.h>

#define PAD(v, p) ((v + (p)-1) & (~((p)-1)))

class JpegConverter::Impl
{
public:
  Impl();
  ~Impl();

  SharedCVData convert(const Image& img);
  unsigned long TJBUFSIZE(unsigned long width, unsigned long height);
  void renewBuffer();

private:
  JSAMPROW row_ptr_;
  jpeg_compress_struct cinfo_;
  jpeg_error_mgr jerr_;
  unsigned char* compressedImage_;
  unsigned long jpegBufSize_;
  unsigned long jpegSize_;
  int jpegQuality_;
  unsigned long curHeight_;
  unsigned long curWidth_;
};


JpegConverter::JpegConverter()
  : pImpl_(new Impl)
{
}

SharedCVData JpegConverter::convert(const Image& img)
{
  return pImpl_->convert(img);
}


JpegConverter::Impl::Impl()
  : compressedImage_(NULL)
  , jpegBufSize_(0)
  , jpegSize_(0)
  , jpegQuality_(75)
  , curHeight_(0)
  , curWidth_(0)
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
  renewBuffer();
}

JpegConverter::Impl::~Impl()
{
  if (compressedImage_ != NULL)
    delete[] compressedImage_;

  jpeg_destroy_compress(&cinfo_);
}


/*
 * @info From the libjpeg-turbo implementation:
 * @url https://github.com/chris-allan/libjpeg-turbo/blob/a91879e159a3ff3cefd6fdd09093f96355d2cb5f/turbojpeg.c
 */
unsigned long JpegConverter::Impl::TJBUFSIZE(unsigned long width, unsigned long height)
{
  unsigned long retval = 0;
  if (width < 1 || height < 1)
    throw std::runtime_error("Wierd widths and heights incoming!");
  // This allows for rare corner cases in which a JPEG image can actually be
  // larger than the uncompressed input (we wouldn't mention it if it hadn't
  // happened before.)
  retval = PAD(width, 16) * PAD(height, 16) * 6 + 2048;
  return retval;
}

void JpegConverter::Impl::renewBuffer()
{
  if (curWidth_ < cinfo_.image_width || curHeight_ < cinfo_.image_height)
  {
    curWidth_ = cinfo_.image_width;
    curHeight_ = cinfo_.image_height;
    delete[] compressedImage_;
    jpegBufSize_ = TJBUFSIZE(curWidth_, curHeight_);
    compressedImage_ = new unsigned char[jpegBufSize_];
  }

  jpegSize_ = jpegBufSize_;
  jpeg_mem_dest(&cinfo_, &compressedImage_, &jpegSize_);
}

SharedCVData JpegConverter::Impl::convert(const Image& img)
{
  cinfo_.image_width = img.size_.x();
  cinfo_.image_height = img.size_.y();
  cinfo_.input_components = 3;

  renewBuffer();

  jpeg_start_compress(&cinfo_, TRUE);

  while (cinfo_.next_scanline < cinfo_.image_height)
  {
    // Maybe room for improvment??
    row_ptr_ = (JSAMPLE*)(&(img.data_[cinfo_.next_scanline * cinfo_.image_width]));
    jpeg_write_scanlines(&cinfo_, &row_ptr_, 1);
  }

  jpeg_finish_compress(&cinfo_);

  return SharedCVData(new CVData(compressedImage_, compressedImage_ + jpegSize_));
}
