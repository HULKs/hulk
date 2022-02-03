#include "TFRecordOffset.hpp"

namespace Hulks::Transformer
{

  TFRecordOffset::TFRecordOffset(std::filesystem::path path, std::ifstream::pos_type offset,
                                 std::ifstream::pos_type length)
    : path{std::move(path)}
    , offset{offset}
    , length{length}
  {
  }

} // namespace Hulks::Transformer
