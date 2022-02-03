#pragma once

#include <Eigen/Dense>

#include "Tools/Storage/UniValue/UniValue.h"


/**
 * @brief writes an Eigen matrix to a Uni::Value
 * @tparam Scalar The type of the elements
 * @tparam RowsAtCompileTime The number of rows of the matrix
 * @tparam ColsAtCompileTime The number of cols of the matrix
 * @tparam Options Mainly describes whether the matrix is row major or column major
 * @tparam MaxRowsAtCompileTime The maximum rows of the matrix
 * @tparam MaxColsAtCompileTime The maximum cols of the matrix
 * @param out The stream to write to
 * @param matrix The matrix to write
 */
template <typename Scalar, int RowsAtCompileTime, int ColsAtCompileTime, int Options,
          int MaxRowsAtCompileTime, int MaxColsAtCompileTime>
inline void operator<<(Uni::Value& out,
                       const Eigen::Matrix<Scalar, RowsAtCompileTime, ColsAtCompileTime, Options,
                                           MaxRowsAtCompileTime, MaxColsAtCompileTime>& matrix)
{

  const std::size_t numberOfRows =
      (RowsAtCompileTime == Eigen::Dynamic) ? matrix.rows() : RowsAtCompileTime;
  const std::size_t numberOfCols =
      (ColsAtCompileTime == Eigen::Dynamic) ? matrix.cols() : ColsAtCompileTime;

  out = Uni::Value(Uni::ValueType::ARRAY);
  if (Options & Eigen::RowMajor)
  {
    out.reserve(numberOfRows);

    for (std::size_t row = 0; row < numberOfRows; row++)
    {
      auto& outRow = out[row];
      outRow = Uni::Value(Uni::ValueType::ARRAY);
      outRow.reserve(numberOfCols);

      for (std::size_t col = 0; col < numberOfCols; col++)
      {
        outRow[col] << matrix(row, col);
      }
    }
  }
  else
  {
    out.reserve(numberOfCols);

    for (std::size_t col = 0; col < numberOfCols; col++)
    {
      auto& outCol = out[col];
      outCol = Uni::Value(Uni::ValueType::ARRAY);
      outCol.reserve(numberOfRows);

      for (std::size_t row = 0; row < numberOfRows; row++)
      {
        outCol[row] << matrix(row, col);
      }
    }
  }
}

/**
 * @brief reads an Eigen matrix from a Uni::Value
 * @tparam Scalar The type of the elements
 * @tparam RowsAtCompileTime The number of rows of the matrix
 * @tparam ColsAtCompileTime The number of cols of the matrix
 * @tparam Options Mainly describes whether the matrix is row major or column major
 * @tparam MaxRowsAtCompileTime The maximum rows of the matrix
 * @tparam MaxColsAtCompileTime The maximum cols of the matrix
 * @param out The stream to read from
 * @param matrix The matrix to read
 */
template <typename Scalar, int RowsAtCompileTime, int ColsAtCompileTime, int Options,
          int MaxRowsAtCompileTime, int MaxColsAtCompileTime>
inline void operator>>(const Uni::Value& in,
                       Eigen::Matrix<Scalar, RowsAtCompileTime, ColsAtCompileTime, Options,
                                     MaxRowsAtCompileTime, MaxColsAtCompileTime>& matrix)
{
#ifndef NDEBUG
  const std::size_t numberOfRows =
      (RowsAtCompileTime == Eigen::Dynamic) ? matrix.rows() : RowsAtCompileTime;
  const std::size_t numberOfCols =
      (ColsAtCompileTime == Eigen::Dynamic) ? matrix.cols() : ColsAtCompileTime;
#endif

  assert(in.type() == Uni::ValueType::ARRAY);
  unsigned int row, col;
  if (Options & Eigen::RowMajor)
  {
    assert(in.size() == numberOfRows);
    row = 0;
    for (auto it = in.vectorBegin(); it != in.vectorEnd(); it++, row++)
    {
      const auto& inRow = *it;
      assert(inRow.type() == Uni::ValueType::ARRAY);
      assert(inRow.size() == numberOfCols);
      col = 0;
      for (auto it2 = inRow.vectorBegin(); it2 != inRow.vectorEnd(); it2++, col++)
      {
        *it2 >> matrix(row, col);
      }
    }
  }
  else
  {
    assert(in.size() == numberOfCols);
    col = 0;
    for (auto it = in.vectorBegin(); it != in.vectorEnd(); it++, col++)
    {
      const auto& inCol = *it;
      assert(inCol.type() == Uni::ValueType::ARRAY);
      assert(inCol.size() == numberOfRows);
      row = 0;
      for (auto it2 = inCol.vectorBegin(); it2 != inCol.vectorEnd(); it2++, row++)
      {
        *it2 >> matrix(row, col);
      }
    }
  }
}

/**
 * @brief write a vertical Eigen vector to a Uni::Value
 * @tparam Scalar The type of the elements
 * @tparam RowsAtCompileTime The number of elements of the vector
 * @tparam Options Mainly describes whether the matrix is row major or column major
 * @tparam MaxRowsAtCompileTime The maximum number of elements of the vector
 * @param out The stream to write to
 * @param matrix The vector to write
 */
template <typename Scalar, int RowsAtCompileTime, int Options, int MaxRowsAtCompileTime>
inline void operator<<(
    Uni::Value& out,
    const Eigen::Matrix<Scalar, RowsAtCompileTime, 1, Options, MaxRowsAtCompileTime, 1>& matrix)
{
  const std::size_t numberOfRows =
      (RowsAtCompileTime == Eigen::Dynamic) ? matrix.cols() : RowsAtCompileTime;
  out = Uni::Value(Uni::ValueType::ARRAY);
  out.reserve(numberOfRows);

  for (std::size_t i = 0; i < numberOfRows; i++)
  {
    out[i] << matrix[i];
  }
}

/**
 * @brief reads a vertical Eigen vector from a Uni::Value
 * @tparam Scalar The type of the elements
 * @tparam RowsAtCompileTime The number of elements of the vector
 * @tparam Options Mainly describes whether the matrix is row major or column major
 * @tparam MaxRowsAtCompileTime The maximum number of elements of the vector
 * @param in The stream to read from
 * @param matrix The vector to read
 */
template <typename Scalar, int RowsAtCompileTime, int Options, int MaxRowsAtCompileTime>
inline void
operator>>(const Uni::Value& in,
           Eigen::Matrix<Scalar, RowsAtCompileTime, 1, Options, MaxRowsAtCompileTime, 1>& matrix)
{
#ifndef NDEBUG
  const std::size_t numberOfRows =
      (RowsAtCompileTime == Eigen::Dynamic) ? matrix.cols() : RowsAtCompileTime;
#endif
  assert(in.type() == Uni::ValueType::ARRAY);
  assert(in.size() == numberOfRows);

  std::size_t i = 0;
  for (auto it = in.vectorBegin(); it != in.vectorEnd(); it++)
  {
    *it >> matrix[i++];
  }
}

/**
 * @brief writes a vertical Eigen vector to a Uni::Value
 * @param out The stream to write to
 * @param matrix The vector to write
 */
template <typename Scalar>
inline void operator<<(Uni::Value& out, const Eigen::AngleAxis<Scalar>& axisangle)
{
  out = Uni::Value(Uni::ValueType::ARRAY);
  out.reserve(4);

  out.at(0) = axisangle.angle();
  out.at(1) = axisangle.axis()[0];
  out.at(2) = axisangle.axis()[1];
  out.at(3) = axisangle.axis()[2];
}

/**
 * @brief writes a vertical Eigen vector from a Uni::Value
 * @tparam Scalar The type of the elements
 * @param in The stream to read from
 * @param matrix The vector to read
 */
template <typename Scalar>
inline void operator>>(const Uni::Value& in, Eigen::AngleAxis<Scalar>& axisangle)
{
  assert(in.type() == Uni::ValueType::ARRAY);
  assert(in.size() == 4);

  const Scalar angle = in.at(std::size_t(0)).asDouble();
  Eigen::Matrix<Scalar, 3, 1> axis(in.at(1).asDouble(), in.at(2).asDouble(), in.at(3).asDouble());

  axisangle = Eigen::AngleAxis<Scalar>(angle, axis);
}

template <typename T>
inline void operator>>(const Uni::Value& in, std::vector<T, Eigen::aligned_allocator<T>>& out)
{
  T value;
  assert(in.type() == Uni::ValueType::ARRAY);
  out.clear();
  out.reserve(in.size());
  for (auto it = in.vectorBegin(); it != in.vectorEnd(); it++)
  {
    *it >> value;
    out.push_back(value);
  }
}

template <typename T>
inline void operator<<(Uni::Value& out, const std::vector<T, Eigen::aligned_allocator<T>>& in)
{
  Uni::Value::valuesVector_t::size_type i = 0;
  out = Uni::Value(Uni::ValueType::ARRAY);
  out.reserve(in.size());
  for (auto it = in.begin(); it != in.end(); it++)
  {
    out[i++] << *it;
  }
}

#include "Tools/Storage/UniValue/UniValueStreaming.hpp"
