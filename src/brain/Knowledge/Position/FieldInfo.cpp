#include <cmath>

#include "FieldInfo.hpp"
#include <Definitions/windows_definition_fix.hpp>

FieldInfo::FieldInfo(const PlayerConfiguration& playerConfiguration, const FieldDimensions& fieldDimensions)
  : playerConfiguration_(playerConfiguration)
{
  const float fieldLength = fieldDimensions.fieldLength;
  const float fieldWidth = fieldDimensions.fieldWidth;
  const float penaltyBoxWidth = fieldDimensions.fieldPenaltyAreaWidth;
  const float penaltyBoxLength = fieldDimensions.fieldPenaltyAreaLength;
  const float centerCircleRadius = fieldDimensions.fieldCenterCircleDiameter * 0.5f;
  // The goal post position is assumed to be in the center of the post, thus two goal post radii have to be added (a.k.a one diameter).
  const float goalPostDistance = fieldDimensions.goalInnerWidth + fieldDimensions.goalPostDiameter;
  const float goalDepth = fieldDimensions.goalDepth;
  const float goalBoxInFieldDistance = fieldLength * 0.5f - penaltyBoxLength;

  // field border
  lines.push_back({{-fieldLength / 2, fieldWidth / 2}, {fieldLength / 2, fieldWidth / 2}});
  lines.push_back({{-fieldLength / 2, -fieldWidth / 2}, {fieldLength / 2, -fieldWidth / 2}});
  lines.push_back({{-fieldLength / 2, fieldWidth / 2}, {-fieldLength / 2, -fieldWidth / 2}});
  lines.push_back({{fieldLength / 2, fieldWidth / 2}, {fieldLength / 2, -fieldWidth / 2}});

  // center line
  lines.push_back({{0, fieldWidth / 2}, {0, -fieldWidth / 2}});

  // center circle as polygon
  for (float phi = 0; phi < 2 * M_PI; phi += M_PI / 8)
  {
    lines.push_back({polar2cartesian({centerCircleRadius, phi}), polar2cartesian({centerCircleRadius, phi + static_cast<float>(M_PI / 8)})});
  }

  // goal box home
  lines.push_back({{-fieldLength / 2, penaltyBoxWidth / 2}, {-goalBoxInFieldDistance, penaltyBoxWidth / 2}});
  lines.push_back({{-fieldLength / 2, -penaltyBoxWidth / 2}, {-goalBoxInFieldDistance, -penaltyBoxWidth / 2}});

  lines.push_back({{-goalBoxInFieldDistance, penaltyBoxWidth / 2}, {-goalBoxInFieldDistance, -penaltyBoxWidth / 2}});

  // goal box away
  lines.push_back({{fieldLength / 2, penaltyBoxWidth / 2}, {goalBoxInFieldDistance, penaltyBoxWidth / 2}});
  lines.push_back({{fieldLength / 2, -penaltyBoxWidth / 2}, {goalBoxInFieldDistance, -penaltyBoxWidth / 2}});

  lines.push_back({{goalBoxInFieldDistance, penaltyBoxWidth / 2}, {goalBoxInFieldDistance, -penaltyBoxWidth / 2}});

  // There are sometimes lines in the goal support structure.
  if (playerConfiguration_.playerNumber == 1)
  {
    lines.push_back({{fieldLength / 2, -goalPostDistance / 2}, {fieldLength / 2 + goalDepth, -goalPostDistance / 2}});
    lines.push_back({{fieldLength / 2, goalPostDistance / 2}, {fieldLength / 2 + goalDepth, goalPostDistance / 2}});
    lines.push_back({{fieldLength / 2 + goalDepth, -goalPostDistance / 2}, {fieldLength / 2 + goalDepth, goalPostDistance / 2}});

    lines.push_back({{-fieldLength / 2, -goalPostDistance / 2}, {-fieldLength / 2 - goalDepth, -goalPostDistance / 2}});
    lines.push_back({{-fieldLength / 2, goalPostDistance / 2}, {-fieldLength / 2 - goalDepth, goalPostDistance / 2}});
    lines.push_back({{-fieldLength / 2 - goalDepth, -goalPostDistance / 2}, {-fieldLength / 2 - goalDepth, goalPostDistance / 2}});
  }

  /// home
  goalPosts.emplace_back(-fieldLength / 2, goalPostDistance / 2);
  goalPosts.emplace_back(-fieldLength / 2, -goalPostDistance / 2);

  /// away
  goalPosts.emplace_back(fieldLength / 2, goalPostDistance / 2);
  goalPosts.emplace_back(fieldLength / 2, -goalPostDistance / 2);
}

Vector2f FieldInfo::polar2cartesian(const Vector2f& polar) const
{
  float r = polar.x();
  float phi = polar.y();
  return {r * std::cos(phi), r * std::sin(phi)};
}
