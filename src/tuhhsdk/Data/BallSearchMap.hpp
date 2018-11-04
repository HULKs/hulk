#pragma once

#include "FieldDimensions.hpp"

#include "Framework/DataType.hpp"

#include "Tools/Math/Angle.hpp"
#include "Tools/Math/Pose.hpp"
#include "Tools/Time.hpp"


struct ProbCell : public Uni::From, public Uni::To
{
  /// How likely it is that the ball is in this cell
  float probability{0};
  /// The probability during last cycle.
  float oldProbability{0};
  /// How old the value is (in cycles)
  uint32_t age{0};
  /// The position if the cell's center on the field.
  Vector2f position;
  /// The indices of the cell in the map.
  Vector2i indices;

  /**
   * Determines if the two cells are identical.
   *
   * @param cell the cell to compare
   * @return boolean
   */
  bool operator==(const ProbCell& cell) const
  {
    return indices.x() == cell.indices.x() && indices.y() == cell.indices.y();
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::ARRAY);
    value.at(0) << probability;
    value.at(1) << static_cast<float>(age);
    value.at(2) << position.x();
    value.at(3) << position.y();
  }

  void fromValue(const Uni::Value& value) override
  {
    float valueRead;
    value.at(0) >> probability;
    value.at(1) >> valueRead;
    age = static_cast<uint32_t>(valueRead);
    value.at(2) >> position.x();
    value.at(3) >> position.y();
  }
};

class BallSearchMap : public DataType<BallSearchMap>
{
public:
  /// the name of this DataType
  DataTypeName name = "BallSearchMap";
  /// The Probability Map containing cols_ times rows_ ProbCells
  std::vector<std::vector<ProbCell>> probabilityMap_{};
  /// A list of pointers to all probability cells that are inside the field.
  std::list<ProbCell*> probabilityList_{};
  /// The amount of rows and cols the map is divided to.
  int rowsCount_{0}, colsCount_{0};
  /// How big the single cells are (meters)
  float cellWidth_{0}, cellLength_{0};
  /// timepoint when the map was unreliable. Will be reset when playing state changes or player is
  /// penalized.
  TimePoint timestampBallSearchMapUnreliable_;

  void reset() override
  {
    // nothing
  }

  /**
   * @brief returns a cell form a given position
   * @param position
   * @return the cell containing the given position
   */
  ProbCell& cellFromPosition(const Vector2f& position)
  {
    auto x = static_cast<int>((position.x() + fieldLength_ / 2.f) / cellLength_) + 1;
    auto y = static_cast<int>((position.y() + fieldWidth_ / 2.f) / cellWidth_) + 1;

    // Cell index can not be more than number of cols or rows minus 1. (Preventing seg faults)
    x = std::min(colsCount_ - 2, std::max(1, x));
    y = std::min(rowsCount_ - 2, std::max(1, y));

    return probabilityMap_[x][y];
  }

  // TODO: Code duplication > 3000
  const ProbCell& cellFromPositionConst(const Vector2f& position) const
  {
    auto x = static_cast<int>((position.x() + fieldLength_ / 2.f) / cellLength_) + 1;
    auto y = static_cast<int>((position.y() + fieldWidth_ / 2.f) / cellWidth_) + 1;

    // Cell index can not be more than number of cols or rows minus 1. (Preventing seg faults)
    x = std::min(colsCount_ - 2, std::max(1, x));
    y = std::min(rowsCount_ - 2, std::max(1, y));

    return probabilityMap_[x][y];
  }

  /**
   * @brief checks if a given cell is in the FOV of a given robot (given by pose and head yaw)
   * @param pose the pose of the robot
   * @param headYaw the head yaw of the robot
   * @param cell the cell to check
   * @param maxBallDetectionRangeSquared
   * @param fovAngle the cameras FOV angle
   * @param maxHeadYaw the maximum angle at which the shoulders are (almost) not visible.
   * @return bool if the cell is in FOV
   */
  bool isCellInFOV(const Pose& pose, const float headYaw, const ProbCell& cell,
                   const float maxBallDetectionRangeSquared, const float fovAngle,
                   const float maxHeadYaw = 50.f * TO_RAD)
  {
    // A cell is considered not to be in FOV if the head yaw is greater than the given limit as the
    // shoulders will probably block the view. It is (currently) not worth the time to calculate if
    // the view to a cell is not blocked by the shoulders.
    if (std::abs(headYaw) > maxHeadYaw)
    {
      return false;
    }
    Vector2f relCellPosition = cell.position - pose.position;
    if (relCellPosition.squaredNorm() < maxBallDetectionRangeSquared)
    {
      const auto relativeCellAngle =
          static_cast<float>(atan2(relCellPosition.y(), relCellPosition.x()));
      const float angleToHeadX = Angle::angleDiff(relativeCellAngle, headYaw + pose.orientation);
      // Cell is in the radius => cell may be in FOV
      if (std::abs(angleToHeadX) < fovAngle * 0.5f)
      {
        return true;
      }
    }
    return false;
  }

  /**
   * @brief Will create all objects needed by this data type.
   * @param fieldDimensions
   */
  void initialize(const Vector2f& fieldDimensions)
  {
    fieldLength_ = fieldDimensions.x();
    fieldWidth_ = fieldDimensions.y();

    // the amount of cells per column / row including the surrounding layer of one cell in each
    // direction (for convolution)
    colsCount_ = 20;
    rowsCount_ = 14;

    cellWidth_ = fieldWidth_ / (float)(rowsCount_ - 2);
    cellLength_ = fieldLength_ / (float)(colsCount_ - 2);

    probabilityMap_.clear();
    probabilityList_.clear();

    // initialize the map with some non random values
    probabilityMap_.reserve(static_cast<unsigned long>(colsCount_));
    for (int x = 0; x < colsCount_; x++)
    {
      std::vector<ProbCell> probCells;
      probCells.reserve(static_cast<unsigned long>(rowsCount_));
      for (int y = 0; y < rowsCount_; y++)
      {
        ProbCell probCell;
        probCell.position.x() =
            ((float)(x - 1) * cellLength_ + 0.5f * cellLength_) - fieldLength_ / 2.f;
        probCell.position.y() =
            ((float)(y - 1) * cellWidth_ + 0.5f * cellWidth_) - fieldWidth_ / 2.f;
        probCell.indices.x() = x;
        probCell.indices.y() = y;
        probCell.probability = 1.f / static_cast<float>(colsCount_ * rowsCount_);
        probCell.oldProbability = probCell.probability;
        probCell.age = static_cast<uint32_t>(1);

        probCells.push_back(probCell);
      }
      probabilityMap_.push_back(probCells);
    }

    // Only add the inner cells to the probCellList
    for (int x = 1; x < colsCount_ - 1; x++)
    {
      for (int y = 1; y < rowsCount_ - 1; y++)
      {
        probabilityList_.push_back(&(probabilityMap_[x][y]));
      }
    }
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["probabilityMap"] << probabilityMap_;
    value["cellWidth"] << cellWidth_;
    value["cellLength"] << cellLength_;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["probabilityMap"] >> probabilityMap_;
    value["cellWidth"] >> cellWidth_;
    value["cellLength"] >> cellLength_;
  }

private:
  /// The field dimensions given in meters.
  float fieldLength_{}, fieldWidth_{};
};
