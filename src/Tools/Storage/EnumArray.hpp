#pragma once

#include <array>

template <typename ValueType, typename IndexType, std::size_t MaxIndex>
struct EnumArray : public std::array<ValueType, MaxIndex>
{
  constexpr ValueType& operator[](IndexType key)
  {
    return std::array<ValueType, MaxIndex>::operator[](static_cast<std::size_t>(key));
  }

  constexpr const ValueType& operator[](IndexType key) const
  {
    return std::array<ValueType, MaxIndex>::operator[](static_cast<std::size_t>(key));
  }
};
