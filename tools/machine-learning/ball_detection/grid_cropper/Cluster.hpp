#pragma once

#include "Circle.hpp"
#include "CircleWithMetadata.hpp"
#include <vector>

namespace Hulks::GridCropper
{

  /// contains the current merged circle of the cluster and all items belonging to cluster
  struct Cluster
  {
    Circle<float> mergedCircle{};
    std::vector<CircleWithMetadata> candidatesInCluster{};

    Cluster(Circle<float> mergedCircle, std::vector<CircleWithMetadata> candidatesInCluster)
      : mergedCircle{mergedCircle}
      , candidatesInCluster{std::move(candidatesInCluster)}
    {
    }
  };

} // namespace Hulks::GridCropper
