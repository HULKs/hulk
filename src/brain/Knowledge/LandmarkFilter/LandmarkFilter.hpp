#pragma once

#include "Data/CycleInfo.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GoalData.hpp"
#include "Data/LandmarkModel.hpp"
#include "Data/OdometryOffset.hpp"
#include "Framework/Module.hpp"
#include "Tools/Storage/UniValue/UniValue.h"
#include "Tools/Time.hpp"

class Brain;

class LandmarkFilter : public Module<LandmarkFilter, Brain>, public Uni::To
{
public:
  /**
   * @brief toValue converts this to a Uni::Value
   * @param value the resulting Uni::Value
   */
  void toValue(Uni::Value& value) const;
  /**
   * @brief LandmarkFilter initializes members
   * @param manager a reference to brain
   */
  LandmarkFilter(const ModuleManagerInterface& manager);
  /**
   * @brief cycle combines vision data to landmarks with filtering
   */
  void cycle();

private:
  struct GoalPost : public Uni::To
  {
    GoalPost(const Vector2f& p, TimePoint t)
      : position(p)
      , timestampLastSeen(t)
    {
    }
    /**
     * @brief toValue converts this to a Uni::Value
     * @param value the resulting Uni::Value
     */
    void toValue(Uni::Value& value) const
    {
      value = Uni::Value(Uni::ValueType::OBJECT);
      value["position"] << position;
      value["timestampLastSeen"] << timestampLastSeen;
    }

    /// the position of the goal post
    Vector2f position;
    /// the timestamp when this goal post was last percepted
    TimePoint timestampLastSeen;
  };

  /**
   * @brief updateGoalPosts updates the goal posts with current goalData_ and move them according to odometry changes
   */
  void updateGoalPosts();
  /**
   * @brief assembleGoals assembles the goals from the percepted goal posts
   */
  void assembleGoals();

  /// switches buffering of goal posts on and off
  const Parameter<bool> bufferGoalPosts_;
  /// the maximum deviation (in meters) of the distance between two goal posts to the optimal distance
  const Parameter<float> maxGoalPostDistanceDeviation_;
  /// the maximum allowed age of a goal post
  const Parameter<int> maxGoalPostAge_;
  /// the maximum allowed distance between two percepted goal posts to still allow merging
  const Parameter<float> goalPostAssociationRadius_;
  /// some information about the cycle this module is running in
  const Dependency<CycleInfo> cycleInfo_;
  /// unfiltered goal result
  const Dependency<GoalData> goalData_;
  /// the field dimensions
  const Dependency<FieldDimensions> fieldDimensions_;
  /// a reference to the odometry offset
  const Dependency<OdometryOffset> odometryOffset_;
  /// filtered landmarks
  Production<LandmarkModel> landmarkModel_;
  /// a buffer for the percepted goal posts
  std::list<GoalPost> goalPostBuffer_;
  /// the optimal distance between the center of two goal posts (according to the map)
  const float optimalGoalPostDistance_;
  /// the timestamp of the last used goal data
  TimePoint lastTimestamp_;
};
