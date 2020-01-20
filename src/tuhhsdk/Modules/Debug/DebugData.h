#pragma once

#include <boost/variant.hpp>
#include <cstdint>
#include <memory>
#include <string>
#include <vector>

#include <Tools/Storage/Image.hpp>
#include <Tools/Time.hpp>


typedef std::vector<uint8_t> CVData;
typedef std::shared_ptr<CVData> SharedCVData;

typedef Uni::Value DebugValueType;

struct DebugData : public Uni::To
{
  DebugData(const std::string& key, const DebugValueType* value,
            const TimePoint& timestamp = TimePoint::getCurrentTime())
    : timestamp(timestamp)
    , key(key)
    , value(value)
  {
  }

  TimePoint timestamp;
  std::string key;
  const DebugValueType* value;

  // Uni::To interface
  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["key"] << key;
    value["value"] << *this->value;
    value["timestamp"] << timestamp;
  }
};
