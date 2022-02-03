#include "Hardware/RobotMetrics.hpp"
#include "Framework/Configuration/Configuration.h"
#include "Framework/Log/Log.hpp"
#include "Hardware/RobotInterface.hpp"
#include "Tools/Math/Angle.hpp"

void RobotMetrics::JointRestriction::fromValue(const Uni::Value& value)
{
  value["angle"] >> angle;
  value["min"] >> min;
  value["max"] >> max;
}

RobotMetrics::RobotMetrics()
  : forwardKinematics_(*this)
  , inverseKinematics_(*this)
  , com_(*this)
{
}

const ForwardKinematics& RobotMetrics::forwardKinematics() const
{
  return forwardKinematics_;
}

const InverseKinematics& RobotMetrics::inverseKinematics() const
{
  return inverseKinematics_;
}

const Com& RobotMetrics::com() const
{
  return com_;
}

float RobotMetrics::mass(const Elements element) const
{
  return masses_[static_cast<std::size_t>(element)];
}

float RobotMetrics::link(const Links link) const
{
  return links_[static_cast<std::size_t>(link)];
}

Vector3f RobotMetrics::com(const Elements element) const
{
  return coms_[static_cast<std::size_t>(element)];
}

Vector2f RobotMetrics::fsrPosition(const FSRs fsr) const
{
  return fsrPositions_[static_cast<std::size_t>(fsr)];
}

float RobotMetrics::minRange(const Joints joint) const
{
  return minJointRanges_[joint];
}

float RobotMetrics::maxRange(const Joints joint) const
{
  return maxJointRanges_[joint];
}

std::pair<float, float> RobotMetrics::interpolate(const std::vector<JointRestriction>& restrictions,
                                                  const float indexAngle)
{
  assert(!std::isnan(indexAngle) && "indexAngle in RobotMetrics interpolation is NaN");
  assert(!restrictions.empty());
  const auto largerThan =
      std::find_if(restrictions.begin(), restrictions.end(), [indexAngle](const auto& restriction) {
        return restriction.angle >= indexAngle;
      });
  if (largerThan == restrictions.begin())
  {
    return std::pair{largerThan->min, largerThan->max};
  }
  if (largerThan == restrictions.end())
  {
    return std::pair{restrictions.back().min, restrictions.back().max};
  }
  const auto mMin = (largerThan->min - std::prev(largerThan)->min) /
                    (largerThan->angle - std::prev(largerThan)->angle);
  const auto mMax = (largerThan->max - std::prev(largerThan)->max) /
                    (largerThan->angle - std::prev(largerThan)->angle);
  return std::pair{std::prev(largerThan)->min + mMin * (indexAngle - std::prev(largerThan)->angle),
                   std::prev(largerThan)->max + mMax * (indexAngle - std::prev(largerThan)->angle)};
}

float RobotMetrics::minRangeHeadPitch(const float headYaw) const
{
  return interpolate(headPitchRestrictions_, headYaw).first;
}

float RobotMetrics::maxRangeHeadPitch(const float headYaw) const
{
  return interpolate(headPitchRestrictions_, headYaw).second;
}

float RobotMetrics::minRangeLAnkleRoll(const float anklePitch) const
{
  return interpolate(leftAnkleRollRestrictions_, anklePitch).first;
}

float RobotMetrics::maxRangeLAnkleRoll(const float anklePitch) const
{
  return interpolate(leftAnkleRollRestrictions_, anklePitch).second;
}

float RobotMetrics::minRangeRAnkleRoll(const float anklePitch) const
{
  return interpolate(rightAnkleRollRestrictions_, anklePitch).first;
}

float RobotMetrics::maxRangeRAnkleRoll(const float anklePitch) const
{
  return interpolate(rightAnkleRollRestrictions_, anklePitch).second;
}

const RobotMetrics::Lengths& RobotMetrics::lengths() const
{
  return lengths_;
}

void RobotMetrics::configure(Configuration& config, const RobotInfo& robotInfo)
{
  Log<M_TUHHSDK>(LogLevel::INFO) << "Configure RobotMetrics...";

  switch (robotInfo.bodyVersion)
  {
    case RobotVersion::V6:
      config.mount("tuhhSDK.RobotMetrics.Body", "body_v_6.json", ConfigurationType::BODY);
      break;
    default:
      Log<M_TUHHSDK>(LogLevel::ERROR) << "Please check my body version, it is not V6.0";
      throw std::runtime_error("Unknown body version");
  }

  switch (robotInfo.headVersion)
  {
    case RobotVersion::V6:
      config.mount("tuhhSDK.RobotMetrics.Head", "head_v_6.json", ConfigurationType::HEAD);
      break;
    default:
      Log<M_TUHHSDK>(LogLevel::ERROR) << "Please check my head version, it is not V6.0";
      throw std::runtime_error("Unknown body version");
  }
  // link dimensions
  {
    const Uni::Value& dimensions = config.get("tuhhSDK.RobotMetrics.Body", "dimensions");
    links_[static_cast<std::size_t>(Links::NECK_OFFSET_Z)] = dimensions["neck_offset_z"].asDouble();
    links_[static_cast<std::size_t>(Links::SHOULDER_OFFSET_Y)] =
        dimensions["shoulder_offset_y"].asDouble();
    links_[static_cast<std::size_t>(Links::SHOULDER_OFFSET_Z)] =
        dimensions["shoulder_offset_z"].asDouble();
    links_[static_cast<std::size_t>(Links::UPPER_ARM_LENGTH)] =
        dimensions["upper_arm_length"].asDouble();
    links_[static_cast<std::size_t>(Links::LOWER_ARM_LENGTH)] =
        dimensions["lower_arm_length"].asDouble();
    links_[static_cast<std::size_t>(Links::HAND_OFFSET_X)] = dimensions["hand_offset_x"].asDouble();
    links_[static_cast<std::size_t>(Links::HAND_OFFSET_Z)] = dimensions["hand_offset_z"].asDouble();
    links_[static_cast<std::size_t>(Links::HIP_OFFSET_Y)] = dimensions["hip_offset_y"].asDouble();
    links_[static_cast<std::size_t>(Links::HIP_OFFSET_Z)] = dimensions["hip_offset_z"].asDouble();
    links_[static_cast<std::size_t>(Links::THIGH_LENGTH)] = dimensions["thigh_length"].asDouble();
    links_[static_cast<std::size_t>(Links::TIBIA_LENGTH)] = dimensions["tibia_length"].asDouble();
    links_[static_cast<std::size_t>(Links::FOOT_HEIGHT)] = dimensions["foot_height"].asDouble();
    links_[static_cast<std::size_t>(Links::ELBOW_OFFSET_Y)] =
        dimensions["elbow_offset_y"].asDouble();
  }
  // fsr positions
  {
    const Uni::Value& fsrPositions = config.get("tuhhSDK.RobotMetrics.Body", "fsr_positions");
    // left
    fsrPositions_[static_cast<std::size_t>(FSRs::L_FRONT_LEFT)] =
        Vector2f{fsrPositions["L_FL"]["x"].asDouble(), fsrPositions["L_FL"]["y"].asDouble()};
    fsrPositions_[static_cast<std::size_t>(FSRs::L_FRONT_RIGHT)] =
        Vector2f{fsrPositions["L_FR"]["x"].asDouble(), fsrPositions["L_FR"]["y"].asDouble()};
    fsrPositions_[static_cast<std::size_t>(FSRs::L_REAR_LEFT)] =
        Vector2f{fsrPositions["L_RL"]["x"].asDouble(), fsrPositions["L_RL"]["y"].asDouble()};
    fsrPositions_[static_cast<std::size_t>(FSRs::L_FRONT_LEFT)] =
        Vector2f{fsrPositions["L_RR"]["x"].asDouble(), fsrPositions["L_RR"]["y"].asDouble()};
    // right
    fsrPositions_[static_cast<std::size_t>(FSRs::R_FRONT_LEFT)] =
        Vector2f{fsrPositions["R_FL"]["x"].asDouble(), fsrPositions["R_FL"]["y"].asDouble()};
    fsrPositions_[static_cast<std::size_t>(FSRs::R_FRONT_RIGHT)] =
        Vector2f{fsrPositions["R_FR"]["x"].asDouble(), fsrPositions["R_FR"]["y"].asDouble()};
    fsrPositions_[static_cast<std::size_t>(FSRs::R_REAR_LEFT)] =
        Vector2f{fsrPositions["R_RL"]["x"].asDouble(), fsrPositions["R_RL"]["y"].asDouble()};
    fsrPositions_[static_cast<std::size_t>(FSRs::R_FRONT_LEFT)] =
        Vector2f{fsrPositions["R_RR"]["x"].asDouble(), fsrPositions["R_RR"]["y"].asDouble()};
  }
  // masses
  {
    // head
    const Uni::Value& headMasses = config.get("tuhhSDK.RobotMetrics.Head", "masses")["head"];
    masses_[static_cast<std::size_t>(Elements::HEAD)] = headMasses["mass"].asFloat();
    coms_[static_cast<std::size_t>(Elements::HEAD)] =
        Vector3f{headMasses["x"].asFloat(), headMasses["y"].asFloat(), headMasses["z"].asFloat()};
    // body
    const Uni::Value& masses = config.get("tuhhSDK.RobotMetrics.Body", "masses");
    // center
    masses_[static_cast<std::size_t>(Elements::NECK)] = masses["neck"]["mass"].asFloat();
    coms_[static_cast<std::size_t>(Elements::NECK)] =
        Vector3f{masses["neck"]["x"].asFloat(), masses["neck"]["y"].asFloat(),
                 masses["neck"]["z"].asFloat()};
    masses_[static_cast<std::size_t>(Elements::TORSO)] = masses["torso"]["mass"].asFloat();
    coms_[static_cast<std::size_t>(Elements::TORSO)] =
        Vector3f{masses["torso"]["x"].asFloat(), masses["torso"]["y"].asFloat(),
                 masses["torso"]["z"].asFloat()};
    // left
    masses_[static_cast<std::size_t>(Elements::L_SHOULDER)] = masses["lshoulder"]["mass"].asFloat();
    coms_[static_cast<std::size_t>(Elements::L_SHOULDER)] =
        Vector3f{masses["lshoulder"]["x"].asFloat(), masses["lshoulder"]["y"].asFloat(),
                 masses["lshoulder"]["z"].asFloat()};
    masses_[static_cast<std::size_t>(Elements::L_BICEP)] = masses["lbicep"]["mass"].asFloat();
    coms_[static_cast<std::size_t>(Elements::L_BICEP)] =
        Vector3f{masses["lbicep"]["x"].asFloat(), masses["lbicep"]["y"].asFloat(),
                 masses["lbicep"]["z"].asFloat()};
    masses_[static_cast<std::size_t>(Elements::L_ELBOW)] = masses["lelbow"]["mass"].asFloat();
    coms_[static_cast<std::size_t>(Elements::L_ELBOW)] =
        Vector3f{masses["lelbow"]["x"].asFloat(), masses["lelbow"]["y"].asFloat(),
                 masses["lelbow"]["z"].asFloat()};
    masses_[static_cast<std::size_t>(Elements::L_FOREARM)] = masses["lforearm"]["mass"].asFloat();
    coms_[static_cast<std::size_t>(Elements::L_FOREARM)] =
        Vector3f{masses["lforearm"]["x"].asFloat(), masses["lforearm"]["y"].asFloat(),
                 masses["lforearm"]["z"].asFloat()};
    masses_[static_cast<std::size_t>(Elements::L_HAND)] = masses["lhand"]["mass"].asFloat();
    coms_[static_cast<std::size_t>(Elements::L_HAND)] =
        Vector3f{masses["lhand"]["x"].asFloat(), masses["lhand"]["y"].asFloat(),
                 masses["lhand"]["z"].asFloat()};
    masses_[static_cast<std::size_t>(Elements::L_PELVIS)] = masses["lpelvis"]["mass"].asFloat();
    coms_[static_cast<std::size_t>(Elements::L_PELVIS)] =
        Vector3f{masses["lpelvis"]["x"].asFloat(), masses["lpelvis"]["y"].asFloat(),
                 masses["lpelvis"]["z"].asFloat()};
    masses_[static_cast<std::size_t>(Elements::L_HIP)] = masses["lhip"]["mass"].asFloat();
    coms_[static_cast<std::size_t>(Elements::L_HIP)] =
        Vector3f{masses["lhip"]["x"].asFloat(), masses["lhip"]["y"].asFloat(),
                 masses["lhip"]["z"].asFloat()};
    masses_[static_cast<std::size_t>(Elements::L_THIGH)] = masses["lthigh"]["mass"].asFloat();
    coms_[static_cast<std::size_t>(Elements::L_THIGH)] =
        Vector3f{masses["lthigh"]["x"].asFloat(), masses["lthigh"]["y"].asFloat(),
                 masses["lthigh"]["z"].asFloat()};
    masses_[static_cast<std::size_t>(Elements::L_TIBIA)] = masses["ltibia"]["mass"].asFloat();
    coms_[static_cast<std::size_t>(Elements::L_TIBIA)] =
        Vector3f{masses["ltibia"]["x"].asFloat(), masses["ltibia"]["y"].asFloat(),
                 masses["ltibia"]["z"].asFloat()};
    masses_[static_cast<std::size_t>(Elements::L_ANKLE)] = masses["lankle"]["mass"].asFloat();
    coms_[static_cast<std::size_t>(Elements::L_ANKLE)] =
        Vector3f{masses["lankle"]["x"].asFloat(), masses["lankle"]["y"].asFloat(),
                 masses["lankle"]["z"].asFloat()};
    masses_[static_cast<std::size_t>(Elements::L_FOOT)] = masses["lfoot"]["mass"].asFloat();
    coms_[static_cast<std::size_t>(Elements::L_FOOT)] =
        Vector3f{masses["lfoot"]["x"].asFloat(), masses["lfoot"]["y"].asFloat(),
                 masses["lfoot"]["z"].asFloat()};
    // right
    masses_[static_cast<std::size_t>(Elements::R_SHOULDER)] = masses["rshoulder"]["mass"].asFloat();
    coms_[static_cast<std::size_t>(Elements::R_SHOULDER)] =
        Vector3f{masses["rshoulder"]["x"].asFloat(), masses["rshoulder"]["y"].asFloat(),
                 masses["rshoulder"]["z"].asFloat()};
    masses_[static_cast<std::size_t>(Elements::R_BICEP)] = masses["rbicep"]["mass"].asFloat();
    coms_[static_cast<std::size_t>(Elements::R_BICEP)] =
        Vector3f{masses["rbicep"]["x"].asFloat(), masses["rbicep"]["y"].asFloat(),
                 masses["rbicep"]["z"].asFloat()};
    masses_[static_cast<std::size_t>(Elements::R_ELBOW)] = masses["relbow"]["mass"].asFloat();
    coms_[static_cast<std::size_t>(Elements::R_ELBOW)] =
        Vector3f{masses["relbow"]["x"].asFloat(), masses["relbow"]["y"].asFloat(),
                 masses["relbow"]["z"].asFloat()};
    masses_[static_cast<std::size_t>(Elements::R_FOREARM)] = masses["rforearm"]["mass"].asFloat();
    coms_[static_cast<std::size_t>(Elements::R_FOREARM)] =
        Vector3f{masses["rforearm"]["x"].asFloat(), masses["rforearm"]["y"].asFloat(),
                 masses["rforearm"]["z"].asFloat()};
    masses_[static_cast<std::size_t>(Elements::R_HAND)] = masses["rhand"]["mass"].asFloat();
    coms_[static_cast<std::size_t>(Elements::R_HAND)] =
        Vector3f{masses["rhand"]["x"].asFloat(), masses["rhand"]["y"].asFloat(),
                 masses["rhand"]["z"].asFloat()};
    masses_[static_cast<std::size_t>(Elements::R_PELVIS)] = masses["rpelvis"]["mass"].asFloat();
    coms_[static_cast<std::size_t>(Elements::R_PELVIS)] =
        Vector3f{masses["rpelvis"]["x"].asFloat(), masses["rpelvis"]["y"].asFloat(),
                 masses["rpelvis"]["z"].asFloat()};
    masses_[static_cast<std::size_t>(Elements::R_HIP)] = masses["rhip"]["mass"].asFloat();
    coms_[static_cast<std::size_t>(Elements::R_HIP)] =
        Vector3f{masses["rhip"]["x"].asFloat(), masses["rhip"]["y"].asFloat(),
                 masses["rhip"]["z"].asFloat()};
    masses_[static_cast<std::size_t>(Elements::R_THIGH)] = masses["rthigh"]["mass"].asFloat();
    coms_[static_cast<std::size_t>(Elements::R_THIGH)] =
        Vector3f{masses["rthigh"]["x"].asFloat(), masses["rthigh"]["y"].asFloat(),
                 masses["rthigh"]["z"].asFloat()};
    masses_[static_cast<std::size_t>(Elements::R_TIBIA)] = masses["rtibia"]["mass"].asFloat();
    coms_[static_cast<std::size_t>(Elements::R_TIBIA)] =
        Vector3f{masses["rtibia"]["x"].asFloat(), masses["rtibia"]["y"].asFloat(),
                 masses["rtibia"]["z"].asFloat()};
    masses_[static_cast<std::size_t>(Elements::R_ANKLE)] = masses["rankle"]["mass"].asFloat();
    coms_[static_cast<std::size_t>(Elements::R_ANKLE)] =
        Vector3f{masses["rankle"]["x"].asFloat(), masses["rankle"]["y"].asFloat(),
                 masses["rankle"]["z"].asFloat()};
    masses_[static_cast<std::size_t>(Elements::R_FOOT)] = masses["rfoot"]["mass"].asFloat();
    coms_[static_cast<std::size_t>(Elements::R_FOOT)] =
        Vector3f{masses["rfoot"]["x"].asFloat(), masses["rfoot"]["y"].asFloat(),
                 masses["rfoot"]["z"].asFloat()};
  }
  // ranges
  {
    const Uni::Value& ranges = config.get("tuhhSDK.RobotMetrics.Body", "ranges");
    // center
    minJointRanges_[Joints::HEAD_YAW] =
        static_cast<float>(ranges["headyaw"]["min"].asDouble() * TO_RAD);
    maxJointRanges_[Joints::HEAD_YAW] =
        static_cast<float>(ranges["headyaw"]["max"].asDouble() * TO_RAD);
    minJointRanges_[Joints::HEAD_PITCH] =
        static_cast<float>(ranges["headpitch"]["min"].asDouble() * TO_RAD);
    maxJointRanges_[Joints::HEAD_PITCH] =
        static_cast<float>(ranges["headpitch"]["max"].asDouble() * TO_RAD);
    // left
    minJointRanges_[Joints::L_SHOULDER_PITCH] =
        static_cast<float>(ranges["lshoulderpitch"]["min"].asDouble() * TO_RAD);
    maxJointRanges_[Joints::L_SHOULDER_PITCH] =
        static_cast<float>(ranges["lshoulderpitch"]["max"].asDouble() * TO_RAD);
    minJointRanges_[Joints::L_SHOULDER_ROLL] =
        static_cast<float>(ranges["lshoulderroll"]["min"].asDouble() * TO_RAD);
    maxJointRanges_[Joints::L_SHOULDER_ROLL] =
        static_cast<float>(ranges["lshoulderroll"]["max"].asDouble() * TO_RAD);
    minJointRanges_[Joints::L_ELBOW_YAW] =
        static_cast<float>(ranges["lelbowyaw"]["min"].asDouble() * TO_RAD);
    maxJointRanges_[Joints::L_ELBOW_YAW] =
        static_cast<float>(ranges["lelbowyaw"]["max"].asDouble() * TO_RAD);
    minJointRanges_[Joints::L_ELBOW_ROLL] =
        static_cast<float>(ranges["lelbowroll"]["min"].asDouble() * TO_RAD);
    maxJointRanges_[Joints::L_ELBOW_ROLL] =
        static_cast<float>(ranges["lelbowroll"]["max"].asDouble() * TO_RAD);
    minJointRanges_[Joints::L_WRIST_YAW] =
        static_cast<float>(ranges["lwristyaw"]["min"].asDouble() * TO_RAD);
    maxJointRanges_[Joints::L_WRIST_YAW] =
        static_cast<float>(ranges["lwristyaw"]["max"].asDouble() * TO_RAD);
    minJointRanges_[Joints::L_HAND] =
        static_cast<float>(ranges["lhand"]["min"].asDouble() * TO_RAD);
    maxJointRanges_[Joints::L_HAND] =
        static_cast<float>(ranges["lhand"]["max"].asDouble() * TO_RAD);
    minJointRanges_[Joints::L_HIP_YAW_PITCH] =
        static_cast<float>(ranges["lhipyawpitch"]["min"].asDouble() * TO_RAD);
    maxJointRanges_[Joints::L_HIP_YAW_PITCH] =
        static_cast<float>(ranges["lhipyawpitch"]["max"].asDouble() * TO_RAD);
    minJointRanges_[Joints::L_HIP_ROLL] =
        static_cast<float>(ranges["lhiproll"]["min"].asDouble() * TO_RAD);
    maxJointRanges_[Joints::L_HIP_ROLL] =
        static_cast<float>(ranges["lhiproll"]["max"].asDouble() * TO_RAD);
    minJointRanges_[Joints::L_HIP_PITCH] =
        static_cast<float>(ranges["lhippitch"]["min"].asDouble() * TO_RAD);
    maxJointRanges_[Joints::L_HIP_PITCH] =
        static_cast<float>(ranges["lhippitch"]["max"].asDouble() * TO_RAD);
    minJointRanges_[Joints::L_KNEE_PITCH] =
        static_cast<float>(ranges["lkneepitch"]["min"].asDouble() * TO_RAD);
    maxJointRanges_[Joints::L_KNEE_PITCH] =
        static_cast<float>(ranges["lkneepitch"]["max"].asDouble() * TO_RAD);
    minJointRanges_[Joints::L_ANKLE_PITCH] =
        static_cast<float>(ranges["lanklepitch"]["min"].asDouble() * TO_RAD);
    maxJointRanges_[Joints::L_ANKLE_PITCH] =
        static_cast<float>(ranges["lanklepitch"]["max"].asDouble() * TO_RAD);
    minJointRanges_[Joints::L_ANKLE_ROLL] =
        static_cast<float>(ranges["lankleroll"]["min"].asDouble() * TO_RAD);
    maxJointRanges_[Joints::L_ANKLE_ROLL] =
        static_cast<float>(ranges["lankleroll"]["max"].asDouble() * TO_RAD);
    // right
    minJointRanges_[Joints::R_HIP_YAW_PITCH] =
        static_cast<float>(ranges["rhipyawpitch"]["min"].asDouble() * TO_RAD);
    maxJointRanges_[Joints::R_HIP_YAW_PITCH] =
        static_cast<float>(ranges["rhipyawpitch"]["max"].asDouble() * TO_RAD);
    minJointRanges_[Joints::R_HIP_ROLL] =
        static_cast<float>(ranges["rhiproll"]["min"].asDouble() * TO_RAD);
    maxJointRanges_[Joints::R_HIP_ROLL] =
        static_cast<float>(ranges["rhiproll"]["max"].asDouble() * TO_RAD);
    minJointRanges_[Joints::R_HIP_PITCH] =
        static_cast<float>(ranges["rhippitch"]["min"].asDouble() * TO_RAD);
    maxJointRanges_[Joints::R_HIP_PITCH] =
        static_cast<float>(ranges["rhippitch"]["max"].asDouble() * TO_RAD);
    minJointRanges_[Joints::R_KNEE_PITCH] =
        static_cast<float>(ranges["rkneepitch"]["min"].asDouble() * TO_RAD);
    maxJointRanges_[Joints::R_KNEE_PITCH] =
        static_cast<float>(ranges["rkneepitch"]["max"].asDouble() * TO_RAD);
    minJointRanges_[Joints::R_ANKLE_PITCH] =
        static_cast<float>(ranges["ranklepitch"]["min"].asDouble() * TO_RAD);
    maxJointRanges_[Joints::R_ANKLE_PITCH] =
        static_cast<float>(ranges["ranklepitch"]["max"].asDouble() * TO_RAD);
    minJointRanges_[Joints::R_ANKLE_ROLL] =
        static_cast<float>(ranges["rankleroll"]["min"].asDouble() * TO_RAD);
    maxJointRanges_[Joints::R_ANKLE_ROLL] =
        static_cast<float>(ranges["rankleroll"]["max"].asDouble() * TO_RAD);
    minJointRanges_[Joints::R_SHOULDER_PITCH] =
        static_cast<float>(ranges["rshoulderpitch"]["min"].asDouble() * TO_RAD);
    maxJointRanges_[Joints::R_SHOULDER_PITCH] =
        static_cast<float>(ranges["rshoulderpitch"]["max"].asDouble() * TO_RAD);
    minJointRanges_[Joints::R_SHOULDER_ROLL] =
        static_cast<float>(ranges["rshoulderroll"]["min"].asDouble() * TO_RAD);
    maxJointRanges_[Joints::R_SHOULDER_ROLL] =
        static_cast<float>(ranges["rshoulderroll"]["max"].asDouble() * TO_RAD);
    minJointRanges_[Joints::R_ELBOW_YAW] =
        static_cast<float>(ranges["relbowyaw"]["min"].asDouble() * TO_RAD);
    maxJointRanges_[Joints::R_ELBOW_YAW] =
        static_cast<float>(ranges["relbowyaw"]["max"].asDouble() * TO_RAD);
    minJointRanges_[Joints::R_ELBOW_ROLL] =
        static_cast<float>(ranges["relbowroll"]["min"].asDouble() * TO_RAD);
    maxJointRanges_[Joints::R_ELBOW_ROLL] =
        static_cast<float>(ranges["relbowroll"]["max"].asDouble() * TO_RAD);
    minJointRanges_[Joints::R_WRIST_YAW] =
        static_cast<float>(ranges["rwristyaw"]["min"].asDouble() * TO_RAD);
    maxJointRanges_[Joints::R_WRIST_YAW] =
        static_cast<float>(ranges["rwristyaw"]["max"].asDouble() * TO_RAD);
    minJointRanges_[Joints::R_HAND] =
        static_cast<float>(ranges["rhand"]["min"].asDouble() * TO_RAD);
    maxJointRanges_[Joints::R_HAND] =
        static_cast<float>(ranges["rhand"]["max"].asDouble() * TO_RAD);
  }
  // lookuptables
  {
    const Uni::Value& lookuptables = config.get("tuhhSDK.RobotMetrics.Body", "lookuptables");
    lookuptables["headpitch"] >> headPitchRestrictions_;
    lookuptables["lankleroll"] >> leftAnkleRollRestrictions_;
    lookuptables["rankleroll"] >> rightAnkleRollRestrictions_;
  }
  // lengths
  {
    lengths_.foreArmLength = links_[static_cast<float>(Links::LOWER_ARM_LENGTH)] +
                             links_[static_cast<float>(Links::HAND_OFFSET_X)];

    /// maximal arm length (shoulder <-> hand distance)
    lengths_.maxArmLength = static_cast<float>(std::sqrt(
        std::pow(links_[static_cast<std::size_t>(Links::UPPER_ARM_LENGTH)], 2) +
        std::pow(lengths_.foreArmLength, 2) -
        2.f * links_[static_cast<std::size_t>(Links::UPPER_ARM_LENGTH)] * lengths_.foreArmLength *
            std::cos(static_cast<float>(M_PI) + maxJointRanges_[Joints::L_ELBOW_ROLL])));

    /// minimal arm length (shoulder <-> hand distance)
    lengths_.minArmLength = static_cast<float>(std::sqrt(
        std::pow(links_[static_cast<std::size_t>(Links::UPPER_ARM_LENGTH)], 2) +
        std::pow(lengths_.foreArmLength, 2) -
        2.f * links_[static_cast<std::size_t>(Links::UPPER_ARM_LENGTH)] * lengths_.foreArmLength *
            std::cos(static_cast<float>(M_PI) + minJointRanges_[Joints::L_ELBOW_ROLL])));

    /// minimal leg length (hip <-> foot distance)
    lengths_.minLegLength = static_cast<float>(
        std::sqrt(pow(links_[static_cast<std::size_t>(Links::TIBIA_LENGTH)], 2) +
                  pow(links_[static_cast<std::size_t>(Links::THIGH_LENGTH)], 2) -
                  2.f * links_[static_cast<std::size_t>(Links::TIBIA_LENGTH)] *
                      links_[static_cast<std::size_t>(Links::THIGH_LENGTH)] *
                      std::cos(static_cast<float>(M_PI) - maxJointRanges_[Joints::L_KNEE_PITCH])));
    /// maximal leg length (hip <-> foot distance)
    lengths_.maxLegLength = links_[static_cast<std::size_t>(Links::TIBIA_LENGTH)] +
                            links_[static_cast<std::size_t>(Links::THIGH_LENGTH)];

    /// max y-position for LElbow
    lengths_.maxLElbowY = std::sin(maxJointRanges_[Joints::L_SHOULDER_ROLL]) *
                          links_[static_cast<std::size_t>(Links::UPPER_ARM_LENGTH)];

    /// minimal y-position for LElbow
    lengths_.minLElbowY = std::sin(minJointRanges_[Joints::L_SHOULDER_ROLL]) *
                          links_[static_cast<std::size_t>(Links::UPPER_ARM_LENGTH)];

    /// maximal y-position for RElbow
    lengths_.maxRElbowY = std::sin(maxJointRanges_[Joints::R_SHOULDER_ROLL]) *
                          links_[static_cast<std::size_t>(Links::UPPER_ARM_LENGTH)];

    /// minimal y-position for RElbow
    lengths_.minRElbowY = std::sin(minJointRanges_[Joints::R_SHOULDER_ROLL]) *
                          links_[static_cast<std::size_t>(Links::UPPER_ARM_LENGTH)];
  }
}
