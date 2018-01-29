#pragma once

#include <Tools/Math/Eigen.hpp>
#include <Tools/Math/Pose.hpp>

struct PositionParticle : public Uni::To
{
  /**
   * @brief PositionParticle creates a new PositionParticle
   * @param initialPose the pose that this particle represents
   * @param clusterID an ID of a cluster that this particle initially belongs to
   */
  PositionParticle(const Pose& initialPose, unsigned int clusterID)
    : pose(initialPose)
    , weight(0)
    , clusterID(clusterID)
  {
  }
  /**
   * @brief replace replaces the pose of a particle with another one (e.g. due to sensor resetting)
   * @param newPose the new pose of the particle
   * @param newID a new ID of a cluster
   */
  void replace(const Pose& newPose, unsigned int newID)
  {
    *this = PositionParticle(newPose, newID);
  }
  /// the pose that this particle represents
  Pose pose;
  /// weight of the particle (corresponds to the probability that this particle represents the correct pose)
  float weight;
  /// an ID to which cluster the particle belongs
  unsigned int clusterID;
  /**
   * @brief toValue converts this to a Uni::Value
   * @param value the resulting Uni::Value
   */
  void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["pose"] << pose;
    value["weight"] << weight;
    value["clusterID"] << clusterID;
  }
};
