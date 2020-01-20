#include "UniValue.h"
#include "print.h"
#include <cmath>

namespace Uni
{
  Value::Value(ValueType t)
    : type_(t)
  {
    switch (t)
    {
      case ValueType::NIL:
        break;
      case ValueType::INT32:
        value_ = int32_t(0);
        break;
      case ValueType::INT64:
        value_ = int64_t(0);
        break;
      case ValueType::REAL:
        value_ = double(0.0);
        break;
      case ValueType::BOOL:
        value_ = false;
        break;
      case ValueType::STRING:
        value_ = std::string();
        break;
      case ValueType::ARRAY:
        value_ = valuesVector_t();
        break;
      case ValueType::OBJECT:
        value_ = valuesStringMap_t();
        break;
    }
  }

  Value::Value(int32_t i)
    : value_(i)
    , type_(ValueType::INT32)
  {
  }

  Value::Value(int64_t i)
    : value_(i)
    , type_(ValueType::INT64)
  {
  }

  Value::Value(double d)
    : type_(ValueType::REAL)
  {
    if (!std::isfinite(d))
    {
      value_ = 0.;
    }
    value_ = d;
  }

  Value::Value(bool b)
    : value_(b)
    , type_(ValueType::BOOL)
  {
  }

  Value::Value(const std::string& s)
    : value_(s)
    , type_(ValueType::STRING)
  {
  }

  Value::Value(const char* s)
    : value_(std::string(s))
    , type_(ValueType::STRING)
  {
  }

  Value::Value(const To& to)
  {
    to.toValue(*this);
  }

  Value& Value::operator[](const char* key)
  {
    if (type_ == ValueType::NIL)
    {
      type_ = ValueType::OBJECT;
      value_ = valuesStringMap_t();
    }
    else if (type_ != ValueType::OBJECT)
    {
      throw std::runtime_error("Can not apply eckige klammer string to non object type!");
    }
    auto& map = std::get<valuesStringMap_t>(value_);
    // use operator[] here to perform an insertion if such key does not already exist
    return map[key];
  }

  const Value& Value::operator[](const char* key) const
  {
    if (type_ != ValueType::OBJECT)
    {
      throw std::runtime_error("Can not apply eckige klammer string to non object type!");
    }
    const auto& map = std::get<valuesStringMap_t>(value_);
    return map.at(key);
  }

  Value& Value::operator[](const std::string& key)
  {
    return (*this)[key.c_str()];
  }

  const Value& Value::operator[](const std::string& key) const
  {
    return (*this)[key.c_str()];
  }

  Value& Value::operator[](Value::valuesVector_t::size_type pos)
  {
    if (type_ == ValueType::NIL)
    {
      type_ = ValueType::ARRAY;
      value_ = valuesVector_t();
    }
    else if (type_ != ValueType::ARRAY)
    {
      throw std::runtime_error("Uni::Value::operator[] is only useful for arrays (value was not)");
    }
    auto& vector = std::get<valuesVector_t>(value_);
    if (pos >= vector.size())
    {
      vector.resize(pos + 1);
    }
    return vector[pos];
  }

  Value& Value::at(Value::valuesVector_t::size_type pos)
  {
    return (*this)[pos];
  }

  const Value& Value::at(Value::valuesVector_t::size_type pos) const
  {
    if (type_ != ValueType::ARRAY)
    {
      throw std::runtime_error("Uni::Value::at is only useful for arrays (value was not)");
    }
    return std::get<valuesVector_t>(value_).at(pos);
  }

  const Value& Value::operator[](Value::valuesVector_t::size_type pos) const
  {
    return this->at(pos);
  }

  ValueType Value::type() const
  {
    return type_;
  }

  int32_t Value::asInt32() const
  {
    switch (type_)
    {
      case ValueType::INT32:
        return std::get<int32_t>(value_);
        break;
      case ValueType::INT64:
        return static_cast<int32_t>(std::get<int64_t>(value_));
        break;
      case ValueType::REAL:
        return static_cast<int32_t>(std::get<double>(value_));
        break;
      default:
        throw std::runtime_error("Value is not convertible to int32_t.");
    }
  }

  int64_t Value::asInt64() const
  {
    switch (type_)
    {
      case ValueType::INT32:
        return static_cast<int64_t>(std::get<int32_t>(value_));
        break;
      case ValueType::INT64:
        return std::get<int64_t>(value_);
        break;
      case ValueType::REAL:
        return static_cast<int64_t>(std::get<double>(value_));
        break;
      default:
        throw std::runtime_error("Value is not convertible to int64_t.");
    }
  }

  double Value::asDouble() const
  {
    switch (type_)
    {
      case ValueType::INT32:
        return static_cast<double>(std::get<int32_t>(value_));
        break;
      case ValueType::INT64:
        return static_cast<double>(std::get<int64_t>(value_));
        break;
      case ValueType::REAL:
        return std::get<double>(value_);
        break;
      default:
        throw std::runtime_error("Value ist kein Cabrio to double.");
    }
  }

  bool Value::asBool() const
  {
    if (type_ != ValueType::BOOL)
    {
      throw std::runtime_error("Value not convertible to bool");
    }
    return std::get<bool>(value_);
  }

  std::string Value::asString() const
  {
    if (type_ != ValueType::STRING)
    {
      throw std::runtime_error("Value not convertible to std::string!");
    }
    return std::get<std::string>(value_);
  }

  void Value::clear()
  {
    std::visit(
        [](auto&& v) {
          using T = std::decay_t<decltype(v)>;
          if constexpr (std::is_same_v<T, valuesStringMap_t> || std::is_same_v<T, valuesVector_t>)
          {
            v.clear();
            return;
          }
          throw std::runtime_error(
              "Uni::Value::clear() is only useful for OBJECT or ARRAY (value was not)");
        },
        value_);
  }

  std::size_t Value::size() const
  {
    return std::visit(
        [](auto&& arg) -> decltype(this->size()) {
          using T = std::decay_t<decltype(arg)>;
          if constexpr (std::is_same_v<T, valuesStringMap_t> || std::is_same_v<T, valuesVector_t>)
          {
            return arg.size();
          }
          throw std::runtime_error(
              "Uni::Value::size() is only useful for OBJECT or ARRAY (value was not)");
        },
        value_);
  }

  void Value::reserve(valuesVector_t::size_type size)
  {
    if (type_ != ValueType::ARRAY)
    {
      throw std::runtime_error("Uni::Value::reserve() is only useful for ARRAY (value was not)");
    }
    std::get<valuesVector_t>(value_).reserve(size);
  }

  bool Value::contains(const std::string& key) const
  {
    if (type_ != ValueType::OBJECT)
    {
      throw std::runtime_error("Uni::Value::contains() is only useful for OBJECT (value was not)");
    }
    return std::get<valuesStringMap_t>(value_).count(key) > 0;
  }

  Value::valuesStringMap_t::const_iterator Value::objectBegin() const
  {
    if (type_ != ValueType::OBJECT)
    {
      throw std::runtime_error(
          "Uni::Value::objectBegin() is only useful for OBJECT (value was not)");
    }
    return std::get<valuesStringMap_t>(value_).begin();
  }

  Value::valuesStringMap_t::iterator Value::objectBegin()
  {
    if (type_ != ValueType::OBJECT)
    {
      throw std::runtime_error(
          "Uni::Value::objectBegin() is only useful for OBJECT (value was not)");
    }
    return std::get<valuesStringMap_t>(value_).begin();
  }

  Value::valuesStringMap_t::const_iterator Value::objectEnd() const
  {
    if (type_ != ValueType::OBJECT)
    {
      throw std::runtime_error("Uni::Value::objectEnd() is only useful for OBJECT (value was not)");
    }
    return std::get<valuesStringMap_t>(value_).end();
  }

  Value::valuesStringMap_t::iterator Value::objectEnd()
  {
    if (type_ != ValueType::OBJECT)
    {
      throw std::runtime_error(
          "Uni::Value::objectEnd() is only useful for objects (value was not)");
    }
    return std::get<valuesStringMap_t>(value_).end();
  }

  Value::valuesVector_t::const_iterator Value::vectorBegin() const
  {
    if (type_ != ValueType::ARRAY)
    {
      throw std::runtime_error(
          "Uni::Value::vectorBegin() is only useful for ARRAY (value was not)");
    }
    return std::get<valuesVector_t>(value_).begin();
  }

  Value::valuesVector_t::iterator Value::vectorBegin()
  {
    if (type_ != ValueType::ARRAY)
    {
      throw std::runtime_error(
          "Uni::Value::vectorBegin() is only useful for ARRAY (value was not)");
    }
    return std::get<valuesVector_t>(value_).begin();
  }

  Value::valuesVector_t::const_iterator Value::vectorEnd() const
  {
    if (type_ != ValueType::ARRAY)
    {
      throw std::runtime_error("Uni::Value::vectorEnd() is only useful for ARRAY (value was not)");
    }
    return std::get<valuesVector_t>(value_).end();
  }

  Value::valuesVector_t::iterator Value::vectorEnd()
  {
    if (type_ != ValueType::ARRAY)
    {
      throw std::runtime_error("Uni::Value::vectorEnd() is only useful for ARRAY (value was not)");
    }
    return std::get<valuesVector_t>(value_).end();
  }
} // namespace Uni
