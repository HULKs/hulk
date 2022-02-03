#pragma once

#include <cmath>

#include "Framework/Configuration/Configuration.h"
#include "Framework/DataType.hpp"
#include "Tools/Math/Eigen.hpp"


class FieldDimensions : public DataType<FieldDimensions>
{
public:
  /// the name of this DataType
  DataTypeName name__{"FieldDimensions"};
  /// the length of the field (A) [m]
  float fieldLength = 0.f;
  /// the width of the field (B) [m]
  float fieldWidth = 0.f;
  /// the width of the field lines (C) [m]
  float fieldLineWidth = 0.f;
  /// the size of the penalty marker (D) [m]
  float fieldPenaltyMarkerSize = 0.f;
  /// the length of the goal box area (E) [m]
  float fieldGoalBoxAreaLength = 0.f;
  /// the width of the goal box area (F) [m]
  float fieldGoalBoxAreaWidth = 0.f;
  /// the length of the penalty area (G) [m]
  float fieldPenaltyAreaLength = 0.f;
  /// the width of the penalty area (H) [m]
  float fieldPenaltyAreaWidth = 0.f;
  /// the distance of the penalty marker to the end of the field (I) [m]
  float fieldPenaltyMarkerDistance = 0.f;
  /// the diameter of the center circle (J) [m]
  float fieldCenterCircleDiameter = 0.f;
  /// the width of the border strip (K) [m]
  float fieldBorderStripWidth = 0.f;
  /// the diameter of each goal post [m]
  float goalPostDiameter = 0.f;
  /// the height of each goal post [m]
  float goalHeight = 0.f;
  /// the distance between the inner points of the goal posts [m]
  float goalInnerWidth = 0.f;
  /// the depth of the goal [m]
  float goalDepth = 0.f;
  /// the diameter of the ball [m]
  float ballDiameter = 0.f;
  /// the diameter of the robot [m]
  float robotDiameter = 0.f;
  /**
   * @brief reset does nothing
   */
  void reset() override {}
  /**
   * @brief isInsideField determines whether a ball is inside the field according to SPL rules
   * @param position a position in field coordinates (i.e. the center of the ball)
   * @param tolerance a tolerance value that the ball is allowed to be outside the field because of
   * the uncertainty in the ball position
   * @return true iff the position is inside the field
   */
  bool isInsideField(const Vector2f& position, const float tolerance) const
  {
    return (std::abs(position.x()) <
            (fieldLength + ballDiameter + fieldLineWidth) * 0.5f + tolerance) &&
           (std::abs(position.y()) <
            (fieldWidth + ballDiameter + fieldLineWidth) * 0.5f + tolerance);
  }
  /**
   * @brief isInsideCarpet determines whether a position is on the carpet
   * @param position a position in field coordinates
   * @return true iff the position is on the carpet
   */
  bool isInsideCarpet(const Vector2f& position) const
  {
    return (std::abs(position.x()) < (fieldLength * 0.5f + fieldBorderStripWidth)) &&
           (std::abs(position.y()) < (fieldWidth * 0.5f + fieldBorderStripWidth));
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["fieldLength"] << fieldLength;
    value["fieldWidth"] << fieldWidth;
    value["fieldLineWidth"] << fieldLineWidth;
    value["fieldPenaltyMarkerSize"] << fieldPenaltyMarkerSize;
    value["fieldGoalBoxAreaLength"] << fieldGoalBoxAreaLength;
    value["fieldGoalBoxAreaWidth"] << fieldGoalBoxAreaWidth;
    value["fieldPenaltyAreaLength"] << fieldPenaltyAreaLength;
    value["fieldPenaltyAreaWidth"] << fieldPenaltyAreaWidth;
    value["fieldPenaltyMarkerDistance"] << fieldPenaltyMarkerDistance;
    value["fieldCenterCircleDiameter"] << fieldCenterCircleDiameter;
    value["fieldBorderStripWidth"] << fieldBorderStripWidth;
    value["goalPostDiameter"] << goalPostDiameter;
    value["goalHeight"] << goalHeight;
    value["goalInnerWidth"] << goalInnerWidth;
    value["goalDepth"] << goalDepth;
    value["ballDiameter"] << ballDiameter;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["fieldLength"] >> fieldLength;
    value["fieldWidth"] >> fieldWidth;
    value["fieldLineWidth"] >> fieldLineWidth;
    value["fieldPenaltyMarkerSize"] >> fieldPenaltyMarkerSize;
    value["fieldGoalBoxAreaLength"] >> fieldGoalBoxAreaLength;
    value["fieldGoalBoxAreaWidth"] >> fieldGoalBoxAreaWidth;
    value["fieldPenaltyAreaLength"] >> fieldPenaltyAreaLength;
    value["fieldPenaltyAreaWidth"] >> fieldPenaltyAreaWidth;
    value["fieldPenaltyMarkerDistance"] >> fieldPenaltyMarkerDistance;
    value["fieldCenterCircleDiameter"] >> fieldCenterCircleDiameter;
    value["fieldBorderStripWidth"] >> fieldBorderStripWidth;
    value["goalPostDiameter"] >> goalPostDiameter;
    value["goalHeight"] >> goalHeight;
    value["goalInnerWidth"] >> goalInnerWidth;
    value["goalDepth"] >> goalDepth;
    value["ballDiameter"] >> ballDiameter;
  }

  /**
   * @brief init loads the field dimensions from a configuration file
   * @param config a reference to the configuration provider
   */
  void init(Configuration& config)
  {
    config.mount("tuhhSDK.FieldDimensions", "map.json", ConfigurationType::HEAD);

    // read field parameters
    auto group = config.get("tuhhSDK.FieldDimensions", "field");

    group["length"] >> fieldLength;
    group["width"] >> fieldWidth;
    group["lineWidth"] >> fieldLineWidth;
    group["penaltyMarkerSize"] >> fieldPenaltyMarkerSize;
    group["goalBoxAreaLength"] >> fieldGoalBoxAreaLength;
    group["goalBoxAreaWidth"] >> fieldGoalBoxAreaWidth;
    group["penaltyAreaLength"] >> fieldPenaltyAreaLength;
    group["penaltyAreaWidth"] >> fieldPenaltyAreaWidth;
    group["penaltyMarkerDistance"] >> fieldPenaltyMarkerDistance;
    group["centerCircleDiameter"] >> fieldCenterCircleDiameter;
    group["borderStripWidth"] >> fieldBorderStripWidth;

    // read goal parameters
    group = config.get("tuhhSDK.FieldDimensions", "goal");

    group["postDiameter"] >> goalPostDiameter;
    group["height"] >> goalHeight;
    group["innerWidth"] >> goalInnerWidth;
    group["depth"] >> goalDepth;

    // read ball parameters
    group = config.get("tuhhSDK.FieldDimensions", "ball");
    group["diameter"] >> ballDiameter;

    // read robot parameters
    group = config.get("tuhhSDK.FieldDimensions", "robot");
    group["diameter"] >> robotDiameter;
  }
};
