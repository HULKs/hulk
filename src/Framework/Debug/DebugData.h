#pragma once

#include "Hardware/Clock.hpp"
#include "Tools/Storage/Image.hpp"
#include <cstdint>
#include <memory>
#include <string>
#include <vector>

using CVData = std::vector<uint8_t>;
using SharedCVData = std::shared_ptr<CVData>;
using DebugValueType = Uni::Value;

struct DebugData : public Uni::To
{
  DebugData(std::string key, const DebugValueType* value, const Clock::time_point& timePoint)
    : timePoint{timePoint}
    , key{std::move(key)}
    , value{value}
  {
  }

  Clock::time_point timePoint;
  std::string key;
  const DebugValueType* value;

  // Uni::To interface
  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["key"] << key;
    value["value"] << *this->value;
    value["timePoint"] << timePoint;
  }
};
