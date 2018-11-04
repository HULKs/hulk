#include <boost/assign/list_of.hpp>

#include "Hardware/RobotInterface.hpp"
#include "Modules/Configuration/Configuration.h"
#include "print.h"

#include "NaoProvider.h"

Vector3f NaoProvider::com_[ELEMENTS::ELEMENTS_MAX];
Vector2f NaoProvider::fsrPositions_[FSRS::FSR_MAX];
float NaoProvider::mass_[ELEMENTS::ELEMENTS_MAX];
float NaoProvider::maxRange_[JOINTS::JOINTS_MAX];
float NaoProvider::minRange_[JOINTS::JOINTS_MAX];
float NaoProvider::links_[LINKS::LINKS_MAX];

NaoProvider::lengths NaoProvider::lengths_;

VecVector3f NaoProvider::lookupHeadPitch_;
VecVector3f NaoProvider::lookupLAnkleRoll_;
VecVector3f NaoProvider::lookupRAnkleRoll_;

void NaoProvider::init(Configuration& config, const NaoInfo& info)
{
  static bool initialized = false;
  if (initialized)
  {
    return;
  }
  print("Initializing NaoProvider", LogLevel::INFO);

  std::string bodyConfigFile;
  std::string headConfigFile;

  /// READ BODY BASE VERSION CONFIG
  if (info.bodyVersion == NaoVersion::V5)
  {
    print("My body is V5.0", LogLevel::INFO);
    bodyConfigFile = "body_v_5.json";
  }
  else if (info.bodyVersion == NaoVersion::V3_3)
  {
    print("My body is V3.3", LogLevel::INFO);
    bodyConfigFile = "body_v_3-3.json";
  }
  else
  {
    bodyConfigFile = "body_v_3-3.json";
    print("Please check my body version, it is neither V5.0 nor V3.3!\n"
          "I will assume V3.3",
          LogLevel::ERROR);
  }


  /// READ HEAD BASE VERSION CONFIG
  if (info.headVersion == NaoVersion::V5)
  {
    print("My head is V5.0", LogLevel::INFO);
    headConfigFile = "head_v_5.json";
  }
  else if (info.headVersion == NaoVersion::V4)
  {
    print("My head is V4.0", LogLevel::INFO);
    headConfigFile = "head_v_4.json";
  }
  else
  {
    headConfigFile = "head_v_4.json";
    print("Please check my head version, it is neither V5.0 nor V4.0!\n"
          "I will assume V4.0",
          LogLevel::ERROR);
  }

  config.mount("tuhhSDK.NaoProvider.Body", bodyConfigFile, ConfigurationType::HEAD);
  config.mount("tuhhSDK.NaoProvider.Head", headConfigFile, ConfigurationType::HEAD);

  /// BODY
  Uni::Value& dimensions = config.get("tuhhSDK.NaoProvider.Body", "dimensions");
  {
    auto itS = LINKS::offsetMap.begin();
    auto itE = LINKS::offsetMap.end();

    for (; itS != itE; itS++)
    {
      links_[itS->first] = dimensions[itS->second].asDouble();
    }
  }

  Uni::Value& fsr_positions = config.get("tuhhSDK.NaoProvider.Body", "fsr_positions");
  {
    auto itS = FSRS::fsrMap.begin();
    auto itE = FSRS::fsrMap.end();

    for (; itS != itE; itS++)
    {
      setFSRPosition(fsr_positions[itS->second], itS->first);
    }
  }

  Uni::Value& body_masses = config.get("tuhhSDK.NaoProvider.Body", "masses");
  {
    auto itS = ELEMENTS::elementsMap.begin();
    auto itE = ELEMENTS::elementsMap.end();

    for (; itS != itE; itS++)
    {
      setMasses(body_masses[itS->second], itS->first);
    }
  }

  Uni::Value& body_ranges = config.get("tuhhSDK.NaoProvider.Body", "ranges");
  {
    auto itS = JOINTS::jointsMap.begin();
    auto itE = JOINTS::jointsMap.end();

    for (; itS != itE; itS++)
    {
      setRanges(body_ranges[itS->second], itS->first);
    }
  }


  /// Fill lookup tables
  Uni::Value& lookuptables = config.get("tuhhSDK.NaoProvider.Body", "lookuptables");

  Uni::Value& lookHeadPitch = lookuptables["headpitch"];
  auto it1 = lookHeadPitch.listBegin();
  for (; it1 != lookHeadPitch.listEnd(); it1++)
  {
    lookupHeadPitch_.push_back(Vector3f((*it1)["angle"].asDouble() * TO_RAD,
                                        (*it1)["min"].asDouble() * TO_RAD,
                                        (*it1)["max"].asDouble() * TO_RAD));
  }

  Uni::Value& lookLAnkleRoll = lookuptables["lankleroll"];
  auto it2 = lookLAnkleRoll.listBegin();
  for (; it2 != lookLAnkleRoll.listEnd(); it2++)
  {
    lookupLAnkleRoll_.push_back(Vector3f((*it2)["angle"].asDouble() * TO_RAD,
                                         (*it2)["min"].asDouble() * TO_RAD,
                                         (*it2)["max"].asDouble() * TO_RAD));
  }

  Uni::Value& lookRAnkleRoll = lookuptables["rankleroll"];
  auto it3 = lookRAnkleRoll.listBegin();
  for (; it3 != lookRAnkleRoll.listEnd(); it3++)
  {
    lookupRAnkleRoll_.push_back(Vector3f((*it3)["angle"].asDouble() * TO_RAD,
                                         (*it3)["min"].asDouble() * TO_RAD,
                                         (*it3)["max"].asDouble() * TO_RAD));
  }

  /// HEAD
  Uni::Value& head_masses = config.get("tuhhSDK.NaoProvider.Head", "masses")["head"];

  setMasses(head_masses, ELEMENTS::HEAD);

  /// fore arm = lower arm + hand
  lengths_.foreArmLength = links_[LINKS::LOWER_ARM_LENGTH] + links_[LINKS::HAND_OFFSET_X];

  /// maximal arm length (shoulder <-> hand distance)
  lengths_.maxArmLength =
      sqrt(pow(links_[LINKS::UPPER_ARM_LENGTH], 2) + pow(lengths_.foreArmLength, 2) -
           2 * links_[LINKS::UPPER_ARM_LENGTH] * lengths_.foreArmLength *
               cos((float)M_PI + maxRange_[JOINTS::L_ELBOW_ROLL]));

  /// minimal arm length (shoulder <-> hand distance)
  lengths_.minArmLength =
      sqrt(pow(links_[LINKS::UPPER_ARM_LENGTH], 2) + pow(lengths_.foreArmLength, 2) -
           2 * links_[LINKS::UPPER_ARM_LENGTH] * lengths_.foreArmLength *
               cos((float)M_PI + minRange_[JOINTS::L_ELBOW_ROLL]));


  /// minimal leg length (hip <-> foot distance)
  lengths_.minLegLength =
      sqrt(pow(links_[LINKS::TIBIA_LENGTH], 2) + pow(links_[LINKS::THIGH_LENGTH], 2) -
           2 * links_[LINKS::TIBIA_LENGTH] * links_[LINKS::THIGH_LENGTH] *
               cos((float)M_PI - maxRange_[JOINTS::L_KNEE_PITCH]));

  /// maximal leg length (hip <-> foot distance)
  lengths_.maxLegLength = links_[LINKS::TIBIA_LENGTH] + links_[LINKS::THIGH_LENGTH];

  /// max y-position for LElbow
  lengths_.maxLElbowY = sin(maxRange_[JOINTS::L_SHOULDER_ROLL]) * links_[LINKS::UPPER_ARM_LENGTH];

  /// minimal y-position for LElbow
  lengths_.minLElbowY = sin(minRange_[JOINTS::L_SHOULDER_ROLL]) * links_[LINKS::UPPER_ARM_LENGTH];

  /// maximal y-position for RElbow
  lengths_.maxRElbowY = sin(maxRange_[JOINTS::R_SHOULDER_ROLL]) * links_[LINKS::UPPER_ARM_LENGTH];

  /// minimal y-position for RElbow
  lengths_.minRElbowY = sin(minRange_[JOINTS::R_SHOULDER_ROLL]) * links_[LINKS::UPPER_ARM_LENGTH];

  initialized = true;
}

void NaoProvider::setMasses(Uni::Value& src, ELEMENTS::ELEMENT eDst)
{
  if (src.type() == Uni::ValueType::NIL)
    return;

  mass_[eDst] = src["mass"].asDouble();
  com_[eDst] = Vector3f(src["x"].asDouble(), src["y"].asDouble(), src["z"].asDouble());
}

void NaoProvider::setRanges(Uni::Value& src, JOINTS::JOINT eDst)
{
  minRange_[eDst] = src["min"].asDouble() * TO_RAD;
  maxRange_[eDst] = src["max"].asDouble() * TO_RAD;
}

void NaoProvider::setFSRPosition(Uni::Value& src, FSRS::FSR eDst)
{
  fsrPositions_[eDst].x() = src["x"].asDouble();
  fsrPositions_[eDst].y() = src["y"].asDouble();
}

/** link **/
float NaoProvider::link(const LINKS::LINK& link)
{
  return links_[link];
}

Vector3f NaoProvider::com(const ELEMENTS::ELEMENT& element)
{
  return com_[element];
}

Vector2f NaoProvider::fsrPosition(const FSRS::FSR& fsr)
{
  return fsrPositions_[fsr];
}

float NaoProvider::minRange(const JOINTS::JOINT& joint)
{
  return minRange_[joint];
}

float NaoProvider::maxRange(const JOINTS::JOINT& joint)
{
  return maxRange_[joint];
}

float NaoProvider::mass(const ELEMENTS::ELEMENT& element)
{
  return mass_[element];
}


/** foreArmLenght **/
float NaoProvider::foreArmLength()
{
  return lengths_.foreArmLength;
}

/** maxArmLength **/
float NaoProvider::maxArmLength()
{
  return lengths_.maxArmLength;
}

/** minArmLength **/
float NaoProvider::minArmLength()
{
  return lengths_.minArmLength;
}

/** minLegLength **/
float NaoProvider::minLegLength()
{
  return lengths_.minLegLength;
}

/** maxLegLength **/
float NaoProvider::maxLegLength()
{
  return lengths_.maxLegLength;
}

/** minLElbowY **/
float NaoProvider::minLElbowY()
{
  return lengths_.minLElbowY;
}

/** maxLElbowY **/
float NaoProvider::maxLElbowY()
{
  return lengths_.maxLElbowY;
}

/** minRElbowY **/
float NaoProvider::minRElbowY()
{
  return lengths_.minRElbowY;
}

/** maxRElbowY **/
float NaoProvider::maxRElbowY()
{
  return lengths_.maxRElbowY;
}

template <typename T, std::size_t POS>
T NaoProvider::interpolate(
    std::vector<Eigen::Matrix<T, 3, 1>, Eigen::aligned_allocator<Eigen::Matrix<T, 3, 1>>>& src,
    const T& value)
{
#ifndef NDEBUG
  if (std::isnan(value))
  {
    throw std::runtime_error("Can not interpolate in NaoProvider::interpolate");
  }
#endif

  if (value <= src.at(0)[0])
    return src.at(0)[POS];
  else if (value >= src.back()[0])
    return src.back()[POS];
  else
  {
    try
    {
      unsigned int i;
      // find fitting segment
      for (i = 0; i < src.size() - 1; i++)
      {
        if (value > src.at(i)[0] && value <= src.at(i + 1)[0])
          break;
      }
      // slope between points
      // TODO: Could be out of range
      float m = (src.at(i + 1)[POS] - src.at(i)[POS]) / (src.at(i + 1)[0] - src.at(i)[0]);

      return src.at(i)[POS] + m * (value - src.at(i)[0]);
    }
    catch (std::out_of_range& e)
    {
      print("NaoProvider::interpolate: " + std::string(e.what()), LogLevel::ERROR);
      return 0;
    }
  }
}

// minimal range for HeadPitch ( depends on HeadYaw )
float NaoProvider::minRangeHeadPitch(const float& headYaw)
{
  return interpolate<float, 1>(lookupHeadPitch_, headYaw);
}

// maximal range for HeadPitch ( depends on HeadYaw )
float NaoProvider::maxRangeHeadPitch(const float& headYaw)
{
  return interpolate<float, 2>(lookupHeadPitch_, headYaw);
}

// minimal range for RAnkleRoll ( depends on RAnklePitch )
float NaoProvider::minRangeRAnkleRoll(const float& anklePitch)
{
  return interpolate<float, 1>(lookupRAnkleRoll_, anklePitch);
}

// maximal range for RAnkleRoll ( depends on RAnklePitch )
float NaoProvider::maxRangeRAnkleRoll(const float& anklePitch)
{
  return interpolate<float, 2>(lookupRAnkleRoll_, anklePitch);
}

// minimal range for LAnkleRoll ( depends on RAnklePitch )
float NaoProvider::minRangeLAnkleRoll(const float& anklePitch)
{
  return interpolate<float, 1>(lookupLAnkleRoll_, anklePitch);
}

// maximal range for LAnkleRoll ( depends on RAnklePitch )
float NaoProvider::maxRangeLAnkleRoll(const float& anklePitch)
{
  return interpolate<float, 2>(lookupLAnkleRoll_, anklePitch);
}


const std::map<const enum ELEMENTS::ELEMENT, const std::string> ELEMENTS::elementsMap =
    boost::assign::map_list_of          //
    (ELEMENTS::HEAD, "head")            //
    (ELEMENTS::NECK, "neck")            //
    (ELEMENTS::TORSO, "torso")          //
    (ELEMENTS::L_SHOULDER, "lshoulder") //
    (ELEMENTS::R_SHOULDER, "rshoulder") //
    (ELEMENTS::L_BICEP, "lbicep")       //
    (ELEMENTS::R_BICEP, "rbicep")       //
    (ELEMENTS::L_ELBOW, "lelbow")       //
    (ELEMENTS::R_ELBOW, "relbow")       //
    (ELEMENTS::L_FOREARM, "lforearm")   //
    (ELEMENTS::R_FOREARM, "rforearm")   //
    (ELEMENTS::L_HAND, "lhand")         //
    (ELEMENTS::R_HAND, "rhand")         //
    (ELEMENTS::L_PELVIS, "lpelvis")     //
    (ELEMENTS::R_PELVIS, "rpelvis")     //
    (ELEMENTS::L_HIP, "lhip")           //
    (ELEMENTS::R_HIP, "rhip")           //
    (ELEMENTS::L_THIGH, "lthigh")       //
    (ELEMENTS::R_THIGH, "rthigh")       //
    (ELEMENTS::L_TIBIA, "ltibia")       //
    (ELEMENTS::R_TIBIA, "rtibia")       //
    (ELEMENTS::L_ANKLE, "lankle")       //
    (ELEMENTS::R_ANKLE, "rankle")       //
    (ELEMENTS::L_FOOT, "lfoot")         //
    (ELEMENTS::R_FOOT, "rfoot");

const std::map<const enum LINKS::LINK, const std::string> LINKS::offsetMap =
    boost::assign::map_list_of                      //
    (LINKS::NECK_OFFSET_Z, "neck_offset_z")         //
    (LINKS::SHOULDER_OFFSET_Y, "shoulder_offset_y") //
    (LINKS::SHOULDER_OFFSET_Z, "shoulder_offset_z") //
    (LINKS::UPPER_ARM_LENGTH, "upper_arm_length")   //
    (LINKS::LOWER_ARM_LENGTH, "lower_arm_length")   //
    (LINKS::HAND_OFFSET_X, "hand_offset_x")         //
    (LINKS::HAND_OFFSET_Z, "hand_offset_z")         //
    (LINKS::HIP_OFFSET_Y, "hip_offset_y")           //
    (LINKS::HIP_OFFSET_Z, "hip_offset_z")           //
    (LINKS::THIGH_LENGTH, "thigh_length")           //
    (LINKS::TIBIA_LENGTH, "tibia_length")           //
    (LINKS::FOOT_HEIGHT, "foot_height")             //
    (LINKS::ELBOW_OFFSET_Y, "elbow_offset_y");

const std::map<const enum JOINTS::JOINT, const std::string> JOINTS::jointsMap =
    boost::assign::map_list_of                   //
    (JOINTS::HEAD_YAW, "headyaw")                //
    (JOINTS::HEAD_PITCH, "headpitch")            //
    (JOINTS::L_SHOULDER_PITCH, "lshoulderpitch") //
    (JOINTS::R_SHOULDER_PITCH, "rshoulderpitch") //
    (JOINTS::L_SHOULDER_ROLL, "lshoulderroll")   //
    (JOINTS::R_SHOULDER_ROLL, "rshoulderroll")   //
    (JOINTS::L_ELBOW_YAW, "lelbowyaw")           //
    (JOINTS::R_ELBOW_YAW, "relbowyaw")           //
    (JOINTS::L_ELBOW_ROLL, "lelbowroll")         //
    (JOINTS::R_ELBOW_ROLL, "relbowroll")         //
    (JOINTS::L_WRIST_YAW, "lwristyaw")           //
    (JOINTS::R_WRIST_YAW, "rwristyaw")           //
    (JOINTS::L_HAND, "lhand")                    //
    (JOINTS::R_HAND, "rhand")                    //
    (JOINTS::L_HIP_YAW_PITCH, "lhipyawpitch")    //
    (JOINTS::R_HIP_YAW_PITCH, "rhipyawpitch")    //
    (JOINTS::L_HIP_ROLL, "lhiproll")             //
    (JOINTS::R_HIP_ROLL, "rhiproll")             //
    (JOINTS::L_HIP_PITCH, "lhippitch")           //
    (JOINTS::R_HIP_PITCH, "rhippitch")           //
    (JOINTS::L_KNEE_PITCH, "lkneepitch")         //
    (JOINTS::R_KNEE_PITCH, "rkneepitch")         //
    (JOINTS::L_ANKLE_PITCH, "lanklepitch")       //
    (JOINTS::R_ANKLE_PITCH, "ranklepitch")       //
    (JOINTS::L_ANKLE_ROLL, "lankleroll")         //
    (JOINTS::R_ANKLE_ROLL, "rankleroll");

const std::map<const enum FSRS::FSR, const std::string> FSRS::fsrMap = boost::assign::map_list_of //
    (FSRS::L_FL, "L_FL")                                                                          //
    (FSRS::L_FR, "L_FR")                                                                          //
    (FSRS::L_RL, "L_RL")                                                                          //
    (FSRS::L_RR, "L_RR")                                                                          //
    (FSRS::R_FL, "R_FL")                                                                          //
    (FSRS::R_FR, "R_FR")                                                                          //
    (FSRS::R_RL, "R_RL")                                                                          //
    (FSRS::R_RR, "R_RR");
