#pragma once

#include <filesystem>
#include <fstream>

namespace Hulks::Transformer
{

  struct TFRecordOffset
  {
    std::filesystem::path path;
    std::ifstream::pos_type offset;
    std::ifstream::pos_type length;

    TFRecordOffset(std::filesystem::path path, std::ifstream::pos_type offset,
                   std::ifstream::pos_type length);
  };

} // namespace Hulks::Transformer
