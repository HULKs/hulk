#include "StreamCompression.h"

#include <iostream>

#include "zlib.h"
#include <chrono>
#include <fstream>
#include <sstream>

#include "Tools/Time.hpp"

using namespace std;

class StreamCompression::Impl
{
public:
  Impl(size_t size);

  void setFolder(string filename)
  {
    filename_ = filename;
  }

  void openStream();
  void writeData(string str);
  void endStream();
  uint32_t getAvailableSpace();

private:
  string filename_;
  string nextFilename_;

  ofstream fo_;
  z_stream strm_;

  size_t size_;
  unsigned int sizeData_;
  unsigned char* outBuf_;
  bool hasData_;
};


StreamCompression::StreamCompression(size_t size)
  : pImpl_(new Impl(size))
  , isOpen_(false)
{
}

StreamCompression::~StreamCompression()
{
  endStream();
}

void StreamCompression::setFolder(string filename)
{
  pImpl_->setFolder(filename);
}

void StreamCompression::openStream()
{
  isOpen_ = true;
  pImpl_->openStream();
}

void StreamCompression::writeData(string str)
{
  if (isOpen_)
    pImpl_->writeData(str);
}

void StreamCompression::endStream()
{
  if (isOpen_)
  {
    isOpen_ = false;
    pImpl_->endStream();
  }
}

uint32_t StreamCompression::getAvailableSpace()
{
  return pImpl_->getAvailableSpace();
}


StreamCompression::Impl::Impl(size_t size)
  : size_(size)
  , outBuf_(new unsigned char[size])
{
  hasData_ = false;
}

void StreamCompression::Impl::openStream()
{
  if (!hasData_)
  {
    stringstream ss;
    ss << filename_ << "_" << TimePoint::getCurrentTime().getSystemTime() << ".z";
    nextFilename_ = ss.str();

    strm_.zalloc = Z_NULL;
    strm_.zfree = Z_NULL;
    strm_.opaque = Z_NULL;
    strm_.avail_out = size_;
    strm_.next_out = outBuf_;

    deflateInit2(&strm_, Z_DEFAULT_COMPRESSION, Z_DEFLATED, (15 + 16), 8, Z_DEFAULT_STRATEGY);
  }
}

void StreamCompression::Impl::writeData(string str)
{
  hasData_ = true;
  strm_.avail_in = str.length() * sizeof(string::value_type);
  strm_.next_in = (unsigned char*)str.data();

  deflate(&strm_, Z_NO_FLUSH);
  sizeData_ = size_ - strm_.avail_out;

  if (strm_.avail_out < size_ / 10)
  {
    endStream();
    openStream();
  }
}

void StreamCompression::Impl::endStream()
{
  if (hasData_)
  {
    strm_.avail_in = 0;
    deflate(&strm_, Z_FINISH);
    sizeData_ = size_ - strm_.avail_out;

    fo_.open(nextFilename_);
    fo_.write((char*)outBuf_, sizeData_);
    fo_.close();

    hasData_ = false;
    deflateEnd(&strm_);
  }
}

uint32_t StreamCompression::Impl::getAvailableSpace()
{
  return strm_.avail_out;
}
