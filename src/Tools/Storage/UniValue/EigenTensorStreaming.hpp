#pragma once

#include "Tools/Storage/UniValue/UniValue.h"
#include <unsupported/Eigen/CXX11/Tensor>

#ifdef HULK_TARGET_NAO
using dim_t = int;
#else
using dim_t = long int;
#endif

template <typename Scalar>
inline void operator>>(const Uni::Value& uni, Eigen::Tensor<Scalar, 1>& tensor)
{
  assert(uni.type() == Uni::ValueType::ARRAY);
  assert(uni.size() == tensor.dimension(0));
  for (int i = 0; i < tensor.dimension(0); i++)
  {
    tensor(i) = uni[i];
  }
}
template <typename Scalar>
inline void operator>>(const Uni::Value& uni, Eigen::Tensor<Scalar, 2>& tensor)
{
  assert(uni.type() == Uni::ValueType::ARRAY);
  assert(uni.size() == tensor.dimension(0));
  for (int i = 0; i < tensor.dimension(0); i++)
  {
    assert(uni[i].type() == Uni::ValueType::ARRAY);
    assert(uni[i].size() == tensor.dimension(1));
    for (int j = 0; j < tensor.dimension(1); j++)
    {
      tensor(i, j) = uni[i][j];
    }
  }
}
template <typename Scalar>
inline void operator>>(const Uni::Value& uni, Eigen::Tensor<Scalar, 3>& tensor)
{
  assert(uni.type() == Uni::ValueType::ARRAY);
  assert(uni.size() == tensor.dimension(0));
  for (int i = 0; i < tensor.dimension(0); i++)
  {
    assert(uni[i].type() == Uni::ValueType::ARRAY);
    assert(uni[i].size() == tensor.dimension(1));
    for (int j = 0; j < tensor.dimension(1); j++)
    {
      assert(uni[i][j].type() == Uni::ValueType::ARRAY);
      assert(uni[i][j].size() == tensor.dimension(2));
      for (int k = 0; k < tensor.dimension(2); k++)
      {
        tensor(i, j, k) = uni[i][j][k];
      }
    }
  }
}
template <typename Scalar>
inline void operator>>(const Uni::Value& uni, Eigen::Tensor<Scalar, 4>& tensor)
{
  assert(uni.type() == Uni::ValueType::ARRAY);
  assert(uni.size() == tensor.dimension(0));
  for (int i = 0; i < tensor.dimension(0); i++)
  {
    assert(uni[i].type() == Uni::ValueType::ARRAY);
    assert(uni[i].size() == tensor.dimension(1));
    for (int j = 0; j < tensor.dimension(1); j++)
    {
      assert(uni[i][j].type() == Uni::ValueType::ARRAY);
      assert(uni[i][j].size() == tensor.dimension(2));
      for (int k = 0; k < tensor.dimension(2); k++)
      {
        assert(uni[i][j][k].type() == Uni::ValueType::ARRAY);
        assert(uni[i][j][k].size() == tensor.dimension(3));
        for (int l = 0; l < tensor.dimension(3); l++)
        {
          tensor(i, j, k, l) = uni[i][j][k][l];
        }
      }
    }
  }
}


template <typename Scalar, dim_t D1>
inline void operator>>(const Uni::Value& uni,
                       Eigen::TensorFixedSize<Scalar, Eigen::Sizes<D1>>& tensor)
{
  assert(uni.type() == Uni::ValueType::ARRAY);
  assert(uni.size() == D1);
  for (int i = 0; i < D1; i++)
  {
    tensor(i) = uni[i].asDouble();
  }
}
template <typename Scalar, dim_t D1, dim_t D2>
inline void operator>>(const Uni::Value& uni,
                       Eigen::TensorFixedSize<Scalar, Eigen::Sizes<D1, D2>>& tensor)
{
  assert(uni.type() == Uni::ValueType::ARRAY);
  assert(uni.size() == D1);
  for (dim_t i = 0; i < D1; i++)
  {
    assert(uni[i].type() == Uni::ValueType::ARRAY);
    assert(uni[i].size() == D2);
    for (dim_t j = 0; j < D2; j++)
    {
      tensor(i, j) = uni[i][j].asDouble();
    }
  }
}

template <typename Scalar, dim_t D1, dim_t D2, dim_t D3, dim_t D4>
inline void operator>>(const Uni::Value& uni,
                       Eigen::TensorFixedSize<Scalar, Eigen::Sizes<D1, D2, D3, D4>>& tensor)
{
  assert(uni.type() == Uni::ValueType::ARRAY);
  assert(uni.size() == D1);
  for (int i = 0; i < D1; i++)
  {
    assert(uni[i].type() == Uni::ValueType::ARRAY);
    assert(uni[i].size() == D2);
    for (int j = 0; j < D2; j++)
    {
      assert(uni[i][j].type() == Uni::ValueType::ARRAY);
      assert(uni[i][j].size() == D3);
      for (int k = 0; k < D3; k++)
      {
        assert(uni[i][j][k].type() == Uni::ValueType::ARRAY);
        assert(uni[i][j][k].size() == D4);
        for (int l = 0; l < D4; l++)
        {
          tensor(i, j, k, l) = uni[i][j][k][l].asDouble();
        }
      }
    }
  }
}
