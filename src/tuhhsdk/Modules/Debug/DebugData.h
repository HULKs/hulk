#pragma once

#include <cstdint>
#include <string>
#include <vector>
#include <memory>
#include <boost/variant.hpp>

#include <Tools/Time.hpp>
#include <Tools/Storage/Image.hpp>


typedef std::vector<uint8_t> CVData;
typedef std::shared_ptr<CVData> SharedCVData;

typedef Uni::Value DebugValueType;

struct DebugData : public Uni::To{
  DebugData()
    : value()
  {}

  DebugData(const std::string& key, const DebugValueType& value) {
    this->key = key;
    this->value = value;
    this->timestamp = TimePoint::getCurrentTime();
  }

  DebugData(const std::string& key)
    : value()
  {
    this->key = key;
    this->timestamp = TimePoint::getCurrentTime();
  }

  TimePoint timestamp;
  std::string key;
  DebugValueType value;

  // Uni::To interface
  virtual void toValue(Uni::Value &value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["key"] << key;
    value["value"] << this->value;
    value["timestamp"] << timestamp;
  }
};
