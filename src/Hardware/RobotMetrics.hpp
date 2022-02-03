#pragma once

#include "Hardware/Definitions.hpp"
#include "Hardware/Kinematics/Com.hpp"
#include "Hardware/Kinematics/ForwardKinematics.hpp"
#include "Hardware/Kinematics/InverseKinematics.hpp"
#include <array>

class Configuration;
struct RobotInfo;

class RobotMetrics
{
public:
  RobotMetrics();

  void configure(Configuration& config, const RobotInfo& robotInfo);

  const ForwardKinematics& forwardKinematics() const;
  const InverseKinematics& inverseKinematics() const;
  const Com& com() const;

  float mass(Elements element) const;
  float link(Links link) const;
  Vector3f com(Elements element) const;
  Vector2f fsrPosition(FSRs fsr) const;
  float minRange(Joints joint) const;
  float maxRange(Joints joint) const;
  float minRangeHeadPitch(float headYaw) const;
  float maxRangeHeadPitch(float headYaw) const;
  float minRangeLAnkleRoll(float anklePitch) const;
  float maxRangeLAnkleRoll(float anklePitch) const;
  float minRangeRAnkleRoll(float anklePitch) const;
  float maxRangeRAnkleRoll(float anklePitch) const;

  struct Lengths
  {
    float foreArmLength{0.f};
    float maxArmLength{0.f};
    float minArmLength{0.f};
    float minLegLength{0.f};
    float maxLegLength{0.f};
    float minLElbowY{0.f};
    float maxLElbowY{0.f};
    float minRElbowY{0.f};
    float maxRElbowY{0.f};
  };
  const Lengths& lengths() const;

private:
  /**
   * @brief JointRestriction describes the restrictions the joint angle should min/max take at given
   * angle
   */
  struct JointRestriction : public Uni::From
  {
    /// the angle at which this restriction holds
    float angle{0.f};
    /// the minimum allowed head pitch
    float min{0.f};
    /// the maximum allowed head pitch
    float max{0.f};

    void fromValue(const Uni::Value& value) override;
  };

  static std::pair<float, float> interpolate(const std::vector<JointRestriction>& restrictions,
                                             float indexAngle);

  ForwardKinematics forwardKinematics_;
  InverseKinematics inverseKinematics_;
  Com com_;

  /// array containing all link lengths [m]
  std::array<float, static_cast<std::size_t>(Links::MAX)> links_{};
  /// the position of the fsr sensors respective to the foot's center [m]
  std::array<Vector2f, static_cast<std::size_t>(FSRs::MAX)> fsrPositions_{};
  /// the individual masses of the robot's elements [kg]
  std::array<float, static_cast<std::size_t>(Elements::MAX)> masses_{};
  /// the position of the com of the individual robot element [m, m, m]
  std::array<Vector3f, static_cast<std::size_t>(Elements::MAX)> coms_{};
  /// the minimum angle value a joint can take
  JointsArray<float> minJointRanges_{};
  /// the maximum angle value a joint can take
  JointsArray<float> maxJointRanges_{};
  /// the restrictions of the head pitch joint
  std::vector<JointRestriction> headPitchRestrictions_;
  /// the restrictions of the left ankle roll joint
  std::vector<JointRestriction> leftAnkleRollRestrictions_;
  /// the restrictions of the right ankle roll joint
  std::vector<JointRestriction> rightAnkleRollRestrictions_;
  /// additional robot lengths
  Lengths lengths_;
};
