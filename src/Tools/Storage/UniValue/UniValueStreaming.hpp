#pragma once

#include "Tools/Storage/UniValue/UniValue.h"

template <typename T>
inline void operator>>(const Uni::Value& in, std::vector<T>& out)
{
  T value;
  assert(in.type() == Uni::ValueType::ARRAY);
  out.clear();
  out.reserve(in.size());
  for (auto it = in.vectorBegin(); it != in.vectorEnd(); it++)
  {
    // NOTE: If you get in trouble here, you most likely fucked up your include order
    *it >> value;
    out.push_back(value);
  }
}

template <typename T>
inline void operator<<(Uni::Value& out, const std::vector<T>& in)
{
  Uni::Value::valuesVector_t::size_type i = 0;
  out = Uni::Value(Uni::ValueType::ARRAY);
  out.reserve(in.size());
  for (const auto& element : in)
  {
    out[i++] << element;
  }
}

template <typename T>
inline void operator>>(const Uni::Value& in, std::set<T>& out)
{
  T value;
  assert(in.type() == Uni::ValueType::ARRAY);
  out.clear();
  for (auto it = in.vectorBegin(); it != in.vectorEnd(); it++)
  {
    // NOTE: If you get in trouble here, you most likely fucked up your include order
    *it >> value;
    out.insert(value);
  }
}

template <typename Key, typename Compare>
inline void operator>>(const Uni::Value& in, std::set<Key, Compare>& out)
{
  Key value;
  assert(in.type() == Uni::ValueType::ARRAY);
  out.clear();
  for (auto it = in.vectorBegin(); it != in.vectorEnd(); it++)
  {
    // NOTE: If you get in trouble here, you most likely fucked up your include order
    *it >> value;
    out.insert(value);
  }
}

template <typename T>
inline void operator<<(Uni::Value& out, const std::set<T>& in)
{
  Uni::Value::valuesVector_t::size_type i = 0;
  out = Uni::Value(Uni::ValueType::ARRAY);
  out.reserve(in.size());
  for (const auto& element : in)
  {
    out[i++] << element;
  }
}

template <typename Key, typename Compare>
inline void operator<<(Uni::Value& out, const std::set<Key, Compare>& in)
{
  Uni::Value::valuesVector_t::size_type i = 0;
  out = Uni::Value(Uni::ValueType::ARRAY);
  out.reserve(in.size());
  for (const auto& element : in)
  {
    out[i++] << element;
  }
}

template <typename T>
inline void operator>>(const Uni::Value& in, std::list<T>& out)
{
  T value;
  assert(in.type() == Uni::ValueType::ARRAY);
  out.clear();
  for (auto it = in.vectorBegin(); it != in.vectorEnd(); it++)
  {
    // NOTE: If you get in trouble here, you most likely fucked up your include order
    *it >> value;
    out.push_back(value);
  }
}

template <typename T>
inline void operator<<(Uni::Value& out, const std::list<T>& in)
{
  Uni::Value::valuesVector_t::size_type i = 0;
  out = Uni::Value(Uni::ValueType::ARRAY);
  out.reserve(in.size());
  for (const auto& element : in)
  {
    out[i++] << element;
  }
}

template <typename T, std::size_t N>
inline void operator>>(const Uni::Value& in, std::array<T, N>& out)
{
  Uni::Value::valuesVector_t::size_type i = 0;
  assert(in.type() == Uni::ValueType::ARRAY);
  assert(in.size() == N);
  for (auto it = in.vectorBegin(); it != in.vectorEnd(); it++)
  {
    // NOTE: If you get in trouble here, you most likely fucked up your include order
    *it >> out[i++];
  }
}

template <typename T, std::size_t N>
inline void operator<<(Uni::Value& out, const std::array<T, N>& in)
{
  Uni::Value::valuesVector_t::size_type i = 0;
  out = Uni::Value(Uni::ValueType::ARRAY);
  out.reserve(N);
  for (const auto& element : in)
  {
    out[i++] << element;
  }
}

template <typename T>
inline void operator>>(const Uni::Value& in, std::pair<T, T>& out)
{
  // explicit operand type is required to avoid ambiguous overload for operator[]
  Uni::Value::valuesVector_t::size_type i = 0;
  assert(in.type() == Uni::ValueType::ARRAY);
  assert(in.size() == 2); // It's a pair.
  in[i++] >> out.first;
  in[i++] >> out.second;
}

template <typename T>
inline void operator<<(Uni::Value& out, const std::pair<T, T>& in)
{
  // explicit operand type is required to avoid ambiguous overload for operator[]
  Uni::Value::valuesVector_t::size_type i = 0;
  out = Uni::Value(Uni::ValueType::ARRAY);
  out.reserve(2); // It's a pair.
  out[i++] << in.first;
  out[i++] << in.second;
}
