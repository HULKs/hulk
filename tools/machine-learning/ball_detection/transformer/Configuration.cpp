#include "Configuration.hpp"

#include <iostream>
#include <regex>

namespace Hulks::Transformer
{

  Configuration::WeightedTFRecordPath::WeightedTFRecordPath(bool considerWeight, float weight,
                                                            std::filesystem::path path)
    : considerWeight{considerWeight}
    , weight{weight}
    , path{std::move(path)}
  {
  }

  Configuration::WeightedTFRecordPath::WeightedTFRecordPath(const std::string& argument)
  {
    std::smatch matches;
    if (!std::regex_match(argument, matches, std::regex{"(?:([0-9]+(?:\\.[0-9]+)?):)?(.+)"}))
    {
      return;
    }

    if (matches.str(1).empty())
    {
      considerWeight = false;
      weight = 0.f;
      path = matches.str(2);
      return;
    }

    errno = 0;
    const auto parsedWeight = std::strtof(matches[1].str().c_str(), nullptr);
    if (errno != 0)
    {
      std::cerr << "Unexpected weight: " << matches[1] << '\n';
      return;
    }

    considerWeight = true;
    weight = parsedWeight;
    path = matches.str(2);
  }

} // namespace Hulks::Transformer
