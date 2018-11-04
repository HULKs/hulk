#pragma once

#include "Tools/Storage/UniValue/EigenTensorStreaming.hpp"
#include "Tools/Storage/UniValue/UniValue.h"
#include "Tools/Storage/UniValue/UniValue2Json.hpp"

#include <Eigen/Dense>
#include <fstream>

/**
 * @brief framework classes for conveniant deserializing of weight matrices and inference of CNNs.
 * @author Georg Felbinger
 */
namespace CNN
{
  /// Dimensions to contract when applying a 2D convolution on a 4D tensor.
  const std::array<Eigen::IndexPair<int>, 1> CONV2D_DIMS = {{Eigen::IndexPair<int>(3, 0)}};
  /// Dimensions to contract when applying a matrix multiplication of 2D tensors.
  const std::array<Eigen::IndexPair<int>, 1> MATMUL_DIMS = {{Eigen::IndexPair<int>(1, 0)}};

  /**
   * @brief Weight gets the values of the weight matrix from the Uni::Value.
   * @tparam Dimensions the Dimensions given by Eigen::Sizes<D1,...>
   * @author Georg Felbinger
   */
  template <typename Dimensions>
  class Weight
  {
  public:
    Weight(const Uni::Value& data, const std::string& name)
    {
      data[name] >> data_;
    }

    const Eigen::TensorFixedSize<float, Dimensions>& operator()() const
    {
      return data_;
    }

  private:
    /// the deserialized tensor.
    Eigen::TensorFixedSize<float, Dimensions> data_;
  };

  /**
   * @brief Classes derived from Graph may contain one or more Weight members which can easily be
   * instansiated. It deserializes a given json file and stores it as an Uni::Value which can be
   * used for the constructor of a Weight.
   * @autho Georg Felbinger
   */
  class Graph
  {
  public:
    Graph(const std::string& jsonFile)
    {
      Json::Reader reader;
      std::ifstream stream(jsonFile);
      if (!stream.good())
      {
        return;
      }
      Json::Value tmp;
      reader.parse(stream, tmp);
      data_ = Uni::Converter::toUniValue(tmp);
    }

  protected:
    /// the deserialized data.
    Uni::Value data_;
  };
} // namespace CNN
