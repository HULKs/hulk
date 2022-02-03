#pragma once

#include "Data/FieldDimensions.hpp"

#include "Framework/DataType.hpp"

#include "Hardware/Clock.hpp"
#include "Tools/Math/Angle.hpp"
#include "Tools/Math/Pose.hpp"


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
  DataTypeName name__{"BallSearchMap"};
  /// The Probability Map containing cols_ times rows_ ProbCells
  // NOLINTNEXTLINE(cppcoreguidelines-non-private-member-variables-in-classes)
  std::vector<std::vector<ProbCell>> probabilityMap{};
  /// A list of pointers to all probability cells that are inside the field.
  // NOLINTNEXTLINE(cppcoreguidelines-non-private-member-variables-in-classes)
  std::list<ProbCell*> probabilityList{};
  /// The amount of rows and cols the map is divided to.
  // NOLINTNEXTLINE(cppcoreguidelines-non-private-member-variables-in-classes)
  int rowsCount{0}, colsCount{0};
  /// How big the single cells are (meters)
  // NOLINTNEXTLINE(cppcoreguidelines-non-private-member-variables-in-classes)
  float cellWidth{0}, cellLength{0};
  /// timepoint when the map was unreliable. Will be reset when playing state changes or player is
  /// penalized.
  // NOLINTNEXTLINE(cppcoreguidelines-non-private-member-variables-in-classes)
  Clock::time_point timestampBallSearchMapUnreliable;

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
    auto x = static_cast<int>((position.x() + fieldLength_ / 2.f) / cellLength) + 1;
    auto y = static_cast<int>((position.y() + fieldWidth_ / 2.f) / cellWidth) + 1;

    // Cell index can not be more than number of cols or rows minus 1. (Preventing seg faults)
    x = std::min(colsCount - 2, std::max(1, x));
    y = std::min(rowsCount - 2, std::max(1, y));

    return probabilityMap[x][y];
  }

  // TODO: Code duplication > 3000
  const ProbCell& cellFromPositionConst(const Vector2f& position) const
  {
    auto x = static_cast<int>((position.x() + fieldLength_ / 2.f) / cellLength) + 1;
    auto y = static_cast<int>((position.y() + fieldWidth_ / 2.f) / cellWidth) + 1;

    // Cell index can not be more than number of cols or rows minus 1. (Preventing seg faults)
    x = std::min(colsCount - 2, std::max(1, x));
    y = std::min(rowsCount - 2, std::max(1, y));

    return probabilityMap[x][y];
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
    Vector2f relCellPosition = cell.position - pose.position();
    if (relCellPosition.squaredNorm() < maxBallDetectionRangeSquared)
    {
      const auto relativeCellAngle =
          static_cast<float>(atan2(relCellPosition.y(), relCellPosition.x()));
      const float angleToHeadX = Angle::angleDiff(relativeCellAngle, headYaw + pose.angle());
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
    colsCount = 20;
    rowsCount = 14;

    cellWidth = fieldWidth_ / static_cast<float>(rowsCount - 2);
    cellLength = fieldLength_ / static_cast<float>(colsCount - 2);

    probabilityMap.clear();
    probabilityList.clear();

    // initialize the map with some non random values
    probabilityMap.reserve(static_cast<std::size_t>(colsCount));
    for (int x = 0; x < colsCount; x++)
    {
      std::vector<ProbCell> probCells;
      probCells.reserve(static_cast<std::size_t>(rowsCount));
      for (int y = 0; y < rowsCount; y++)
      {
        ProbCell probCell;
        probCell.position.x() =
            (static_cast<float>(x - 1) * cellLength + 0.5f * cellLength) - fieldLength_ / 2.f;
        probCell.position.y() =
            (static_cast<float>(y - 1) * cellWidth + 0.5f * cellWidth) - fieldWidth_ / 2.f;
        probCell.indices.x() = x;
        probCell.indices.y() = y;
        probCell.probability = 1.f / static_cast<float>(colsCount * rowsCount);
        probCell.oldProbability = probCell.probability;
        probCell.age = static_cast<uint32_t>(1);

        probCells.push_back(probCell);
      }
      probabilityMap.push_back(probCells);
    }

    // Only add the inner cells to the probCellList
    for (int x = 1; x < colsCount - 1; x++)
    {
      for (int y = 1; y < rowsCount - 1; y++)
      {
        probabilityList.push_back(&(probabilityMap[x][y]));
      }
    }
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["probabilityMap"] << probabilityMap;
    value["cellWidth"] << cellWidth;
    value["cellLength"] << cellLength;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["probabilityMap"] >> probabilityMap;
    value["cellWidth"] >> cellWidth;
    value["cellLength"] >> cellLength;
  }

private:
  /// The field dimensions given in meters.
  float fieldLength_{}, fieldWidth_{};
};
