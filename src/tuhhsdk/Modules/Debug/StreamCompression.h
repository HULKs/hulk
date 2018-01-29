#pragma once

#include <memory>
#include <string>

class StreamCompression
{
public:
  StreamCompression(size_t size);
  ~StreamCompression();

  void setFolder(std::string filename);

  void openStream();
  void writeData(std::string str);
  uint32_t getAvailableSpace();
  void endStream();

private:
  class Impl;

  std::shared_ptr<Impl> pImpl_;
  bool isOpen_;
};
