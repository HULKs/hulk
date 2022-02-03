#include "Hardware/Nao/NaoInterface.hpp"
#include "Data/IMUSensorData.hpp"
#include "Framework/Configuration/Configuration.h"
#include "Framework/Log/Log.hpp"
#include "Tools/Math/Eigen.hpp"
#include <array>
#include <boost/asio/buffer.hpp>
#include <boost/system/detail/errc.hpp>
#include <boost/system/errc.hpp>
#include <boost/system/system_error.hpp>
#include <filesystem>
#include <functional>
#include <iostream>
#include <memory>
#include <mntent.h>
#include <numeric>
#include <thread>

NaoInterface::NaoInterface()
  : socket_{ioContext_}
  , proxyEndpoint_{"/tmp/hula"}
  , topCamera_(CameraPosition::TOP)
  , bottomCamera_(CameraPosition::BOTTOM)
  , currentCamera_(CameraPosition::TOP)
{
  socket_.connect(proxyEndpoint_);
  if (socket_.receive(boost::asio::buffer(&stateStorage_, sizeof(stateStorage_))) !=
      sizeof(stateStorage_))
  {
    Log<M_TUHHSDK>{LogLevel::ERROR} << "socket_.receive() != sizeof(stateStorage_)";
    throw boost::system::system_error{
        boost::system::errc::make_error_code(boost::system::errc::protocol_not_supported)};
  }
  robotConfiguration_ = stateStorage_.robotConfiguration;
}

void NaoInterface::configure(Configuration& config)
{
  initializeRobotConfiguration(config);
  config.setNaoHeadName(robotInfo_.headName);
  config.setNaoBodyName(robotInfo_.bodyName);
  // This needs to be done here because now the identity of the NAO is known.
  topCamera_.configure(config);
  bottomCamera_.configure(config);
  robotMetrics_.configure(config, robotInfo_);
}

std::pair<std::string, bool> NaoInterface::getDataRootAndUSBStickState() const
{
  std::unique_ptr<FILE, std::function<void(FILE*)>> filesystemDescriptionFile{
      setmntent("/proc/mounts", "r"), [](FILE* filesystemDescriptionFile) {
        if (filesystemDescriptionFile != nullptr)
        {
          endmntent(filesystemDescriptionFile);
        }
      }};
  if (!filesystemDescriptionFile)
  {
    Log<M_TUHHSDK>(LogLevel::ERROR) << "Could not get mountpoints for FileTransport directory";
    return {getFileRoot(), false};
  }

  mntent* entry{nullptr};
  mntent dummy{};
  std::array<char, 4096> buffer{};
  while ((entry = getmntent_r(filesystemDescriptionFile.get(), &dummy, buffer.data(),
                              buffer.size())) != nullptr)
  {
    if (std::string{entry->mnt_fsname} == "/dev/sda1")
    {
      auto dataRoot{std::string{entry->mnt_dir} + "/logs/"};
      Log<M_TUHHSDK>(LogLevel::FANCY) << "Will use " << dataRoot << " as FileTransport directory";
      return {std::move(dataRoot), true};
    }
  }

  return {getFileRoot(), false};
}

static void fillJointsArray(const JointsArray<float>& from, ProxyInterface::JointsArray& to)
{
  to.headYaw = from[Joints::HEAD_YAW];
  to.headPitch = from[Joints::HEAD_PITCH];
  to.leftShoulderPitch = from[Joints::L_SHOULDER_PITCH];
  to.leftShoulderRoll = from[Joints::L_SHOULDER_ROLL];
  to.leftElbowYaw = from[Joints::L_ELBOW_YAW];
  to.leftElbowRoll = from[Joints::L_ELBOW_ROLL];
  to.leftWristYaw = from[Joints::L_WRIST_YAW];
  to.leftHipYawPitch = from[Joints::L_HIP_YAW_PITCH];
  to.leftHipRoll = from[Joints::L_HIP_ROLL];
  to.leftHipPitch = from[Joints::L_HIP_PITCH];
  to.leftKneePitch = from[Joints::L_KNEE_PITCH];
  to.leftAnklePitch = from[Joints::L_ANKLE_PITCH];
  to.leftAnkleRoll = from[Joints::L_ANKLE_ROLL];
  to.rightHipRoll = from[Joints::R_HIP_ROLL];
  to.rightHipPitch = from[Joints::R_HIP_PITCH];
  to.rightKneePitch = from[Joints::R_KNEE_PITCH];
  to.rightAnklePitch = from[Joints::R_ANKLE_PITCH];
  to.rightAnkleRoll = from[Joints::R_ANKLE_ROLL];
  to.rightShoulderPitch = from[Joints::R_SHOULDER_PITCH];
  to.rightShoulderRoll = from[Joints::R_SHOULDER_ROLL];
  to.rightElbowYaw = from[Joints::R_ELBOW_YAW];
  to.rightElbowRoll = from[Joints::R_ELBOW_ROLL];
  to.rightWristYaw = from[Joints::R_WRIST_YAW];
  to.leftHand = from[Joints::L_HAND];
  to.rightHand = from[Joints::R_HAND];
}

void NaoInterface::setJointAngles(const JointsArray<float>& angles)
{
  fillJointsArray(angles, controlStorage_.position);
}

void NaoInterface::setJointStiffnesses(const JointsArray<float>& stiffnesses)
{
  fillJointsArray(stiffnesses, controlStorage_.stiffness);
}

static void fillColor(const Led::Color& from, ProxyInterface::Color& to)
{
  to.red = from.red;
  to.green = from.green;
  to.blue = from.blue;
}

static void fillEar(const Led::Ear& from, ProxyInterface::Ear& to)
{
  to.intensityAt0 = from.intensityAt0;
  to.intensityAt36 = from.intensityAt36;
  to.intensityAt72 = from.intensityAt72;
  to.intensityAt108 = from.intensityAt108;
  to.intensityAt144 = from.intensityAt144;
  to.intensityAt180 = from.intensityAt180;
  to.intensityAt216 = from.intensityAt216;
  to.intensityAt252 = from.intensityAt252;
  to.intensityAt288 = from.intensityAt288;
  to.intensityAt324 = from.intensityAt324;
}

static void fillEye(const Led::Eye& from, ProxyInterface::Eye& to)
{
  fillColor(from.colorAt0, to.colorAt0);
  fillColor(from.colorAt45, to.colorAt45);
  fillColor(from.colorAt90, to.colorAt90);
  fillColor(from.colorAt135, to.colorAt135);
  fillColor(from.colorAt180, to.colorAt180);
  fillColor(from.colorAt225, to.colorAt225);
  fillColor(from.colorAt270, to.colorAt270);
  fillColor(from.colorAt315, to.colorAt315);
}

void NaoInterface::setLEDs(const Led::Chest& chest, const Led::Ear& leftEar,
                           const Led::Ear& rightEar, const Led::Eye& leftEye,
                           const Led::Eye& rightEye, const Led::Foot& leftFoot,
                           const Led::Foot& rightFoot)
{
  fillColor(chest.color, controlStorage_.chest);
  fillEar(leftEar, controlStorage_.leftEar);
  fillEar(rightEar, controlStorage_.rightEar);
  fillEye(leftEye, controlStorage_.leftEye);
  fillEye(rightEye, controlStorage_.rightEye);
  fillColor(leftFoot.color, controlStorage_.leftFoot);
  fillColor(rightFoot.color, controlStorage_.rightFoot);
}

static void fillIMUSensorData(const ProxyInterface::InertialMeasurementUnit& from,
                              IMUSensorData& to)
{
  to.accelerometer.x() = from.accelerometer.x;
  to.accelerometer.y() = from.accelerometer.y;
  to.accelerometer.z() = from.accelerometer.z;
  to.angle.x() = from.angles.x;
  to.angle.y() = from.angles.y;
  to.gyroscope.x() = from.gyroscope.x;
  to.gyroscope.y() = from.gyroscope.y;
  to.gyroscope.z() = from.gyroscope.z;
}

static void fillFSRSensorData(const ProxyInterface::ForceSensitiveResistors& from,
                              FSRSensorData& to)
{
  to.leftFoot.frontLeft = from.leftFootFrontLeft;
  to.leftFoot.frontRight = from.leftFootFrontRight;
  to.leftFoot.rearLeft = from.leftFootRearLeft;
  to.leftFoot.rearRight = from.leftFootRearRight;
  to.rightFoot.frontLeft = from.rightFootFrontLeft;
  to.rightFoot.frontRight = from.rightFootFrontRight;
  to.rightFoot.rearLeft = from.rightFootRearLeft;
  to.rightFoot.rearRight = from.rightFootRearRight;
}

static void fillButtonData(const ProxyInterface::TouchSensors& from, ButtonData& to)
{
  to.switches.isChestButtonPressed = from.chestButton;
  to.switches.isHeadFrontPressed = from.headFront;
  to.switches.isHeadMiddlePressed = from.headMiddle;
  to.switches.isHeadRearPressed = from.headRear;
  to.switches.isLeftFootLeftPressed = from.leftFootLeft;
  to.switches.isLeftFootRightPressed = from.leftFootRight;
  to.switches.isLeftHandBackPressed = from.leftHandBack;
  to.switches.isLeftHandLeftPressed = from.leftHandLeft;
  to.switches.isLeftHandRightPressed = from.leftHandRight;
  to.switches.isRightFootLeftPressed = from.rightFootLeft;
  to.switches.isRightFootRightPressed = from.rightFootRight;
  to.switches.isRightHandBackPressed = from.rightHandBack;
  to.switches.isRightHandLeftPressed = from.rightHandLeft;
  to.switches.isRightHandRightPressed = from.rightHandRight;
}

static void fillSonarSensorData(const ProxyInterface::SonarSensors& from, SonarSensorData& to)
{
  to.data.leftSensor = from.left;
  to.data.rightSensor = from.right;
}

static void fillJointsArray(const ProxyInterface::JointsArray& from, JointsArray<float>& to)
{
  to[Joints::HEAD_YAW] = from.headYaw;
  to[Joints::HEAD_PITCH] = from.headPitch;
  to[Joints::L_SHOULDER_PITCH] = from.leftShoulderPitch;
  to[Joints::L_SHOULDER_ROLL] = from.leftShoulderRoll;
  to[Joints::L_ELBOW_YAW] = from.leftElbowYaw;
  to[Joints::L_ELBOW_ROLL] = from.leftElbowRoll;
  to[Joints::L_WRIST_YAW] = from.leftWristYaw;
  to[Joints::L_HIP_YAW_PITCH] = from.leftHipYawPitch;
  to[Joints::L_HIP_ROLL] = from.leftHipRoll;
  to[Joints::L_HIP_PITCH] = from.leftHipPitch;
  to[Joints::L_KNEE_PITCH] = from.leftKneePitch;
  to[Joints::L_ANKLE_PITCH] = from.leftAnklePitch;
  to[Joints::L_ANKLE_ROLL] = from.leftAnkleRoll;
  to[Joints::R_HIP_ROLL] = from.rightHipRoll;
  to[Joints::R_HIP_PITCH] = from.rightHipPitch;
  to[Joints::R_KNEE_PITCH] = from.rightKneePitch;
  to[Joints::R_ANKLE_PITCH] = from.rightAnklePitch;
  to[Joints::R_ANKLE_ROLL] = from.rightAnkleRoll;
  to[Joints::R_SHOULDER_PITCH] = from.rightShoulderPitch;
  to[Joints::R_SHOULDER_ROLL] = from.rightShoulderRoll;
  to[Joints::R_ELBOW_YAW] = from.rightElbowYaw;
  to[Joints::R_ELBOW_ROLL] = from.rightElbowRoll;
  to[Joints::R_WRIST_YAW] = from.rightWristYaw;
  to[Joints::L_HAND] = from.leftHand;
  to[Joints::R_HAND] = from.rightHand;
}

void NaoInterface::produceSensorData(CycleInfo& cycleInfo, FSRSensorData& fsrSensorData,
                                     IMUSensorData& imuSensorData, JointSensorData& jointSensorData,
                                     ButtonData& buttonData, SonarSensorData& sonarSensorData)
{
  if (socket_.send(boost::asio::buffer(&controlStorage_, sizeof(controlStorage_))) !=
      sizeof(controlStorage_))
  {
    Log<M_TUHHSDK>{LogLevel::ERROR} << "socket_.send() != sizeof(controlStorage_)";
    throw boost::system::system_error{
        boost::system::errc::make_error_code(boost::system::errc::protocol_not_supported)};
  }

  if (socket_.receive(boost::asio::buffer(&stateStorage_, sizeof(stateStorage_))) !=
      sizeof(stateStorage_))
  {
    Log<M_TUHHSDK>{LogLevel::ERROR} << "socket_.receive() != sizeof(stateStorage_)";
    throw boost::system::system_error{
        boost::system::errc::make_error_code(boost::system::errc::protocol_not_supported)};
  }

  static_assert(std::is_same_v<Clock::duration::period, std::chrono::seconds::period>);
  // we will not use stateStorage_.receivedAt since it is based on a different epoch
  cycleInfo.startTime = Clock::time_point{std::chrono::duration_cast<Clock::duration>(
      std::chrono::steady_clock::now().time_since_epoch())};
  cycleInfo.cycleTime = cycleInfo.startTime - lastSensorDataProduction_;
  cycleInfo.valid = true;
  lastSensorDataProduction_ = cycleInfo.startTime;

  fillIMUSensorData(stateStorage_.inertialMeasurementUnit, imuSensorData);
  fillFSRSensorData(stateStorage_.forceSensitiveResistors, fsrSensorData);
  fillButtonData(stateStorage_.touchSensors, buttonData);
  fillSonarSensorData(stateStorage_.sonarSensors, sonarSensorData);
  fillJointsArray(stateStorage_.position, jointSensorData.angles);
  fillJointsArray(stateStorage_.stiffness, jointSensorData.stiffnesses);
  fillJointsArray(stateStorage_.current, jointSensorData.currents);
  fillJointsArray(stateStorage_.temperature, jointSensorData.temperatures);
  fillJointsArray(stateStorage_.status, jointSensorData.status);

  // calculate other values
  fsrSensorData.totalLeft = fsrSensorData.leftFoot.frontLeft + fsrSensorData.leftFoot.frontRight +
                            fsrSensorData.leftFoot.rearLeft + fsrSensorData.leftFoot.rearRight;
  fsrSensorData.rightFoot = fsrSensorData.rightFoot;
  fsrSensorData.totalRight = fsrSensorData.rightFoot.frontLeft +
                             fsrSensorData.rightFoot.frontRight + fsrSensorData.rightFoot.rearLeft +
                             fsrSensorData.rightFoot.rearRight;
  fsrSensorData.valid = true;

  imuSensorData.valid = true;

  jointSensorData.valid = true;

  bool singlePressDetected{!buttonData.switches.isChestButtonPressed &&
                           chestButtonWasPressedLastCycle_};
  if (singlePressDetected)
  {
    lastChestButtonSinglePress_ = cycleInfo.startTime;
  }
  const auto headButtonsPressed{buttonData.switches.isHeadFrontPressed &&
                                buttonData.switches.isHeadMiddlePressed &&
                                buttonData.switches.isHeadRearPressed};
  if (headButtonsPressed && !headButtonsWerePressedLastCycle_)
  {
    headButtonsPressStart_ = cycleInfo.startTime;
  }
  else if (!headButtonsPressed && headButtonsWerePressedLastCycle_)
  {
    headButtonsPressStart_.reset();
  }
  if (headButtonsPressStart_.has_value() &&
      cycleInfo.getAbsoluteTimeDifference(*headButtonsPressStart_) > 100ms)
  {
    lastHeadButtonsHold_ = cycleInfo.startTime;
    headButtonsPressStart_.reset();
  }
  buttonData.lastChestButtonSinglePress = lastChestButtonSinglePress_;
  buttonData.lastHeadButtonsHold = lastHeadButtonsHold_;
  buttonData.valid = true;
  chestButtonWasPressedLastCycle_ = buttonData.switches.isChestButtonPressed;
  headButtonsWerePressedLastCycle_ = headButtonsPressed;

  /// the maximum echo range in meters for the sonar sensors, taken from
  /// http://doc.aldebaran.com/2-1/family/nao_dcm/actuator_sensor_names.html#sonars
  constexpr auto maxSonarRange{5.f};
  // A value <= 0 less means error, >= MAX_DETECTION_RANGE means no echo. Source:
  // http://doc.aldebaran.com/2-1/family/nao_dcm/actuator_sensor_names.html#term-us-sensors-m
  sonarSensorData.valid[Sonars::LEFT] =
      sonarSensorData.data.leftSensor > 0.f && sonarSensorData.data.leftSensor < maxSonarRange;
  sonarSensorData.valid[Sonars::RIGHT] =
      sonarSensorData.data.rightSensor > 0.f && sonarSensorData.data.rightSensor < maxSonarRange;
}

void NaoInterface::enableImageDataProducer()
{
  topCamera_.startCapture();
  bottomCamera_.startCapture();
}

void NaoInterface::disableImageDataProducer()
{
  topCamera_.stopCapture();
  bottomCamera_.stopCapture();
}

void NaoInterface::produceImageData(CycleInfo& cycleInfo, ImageData& imageData)
{
  // Release last used image
  if (currentCamera_ == CameraPosition::TOP)
  {
    topCamera_.releaseImage();
  }
  else if (currentCamera_ == CameraPosition::BOTTOM)
  {
    bottomCamera_.releaseImage();
  }

  const bool imageAvailable = topCamera_.isImageValid() || bottomCamera_.isImageValid();
  // get new images ONLY if there is no valid one anymore.
  if (!imageAvailable)
  {
    bool allImagesAvailable = topCamera_.isImageValid() && bottomCamera_.isImageValid();
    // Wait for all images to become valid
    do
    {
      std::array<NaoCamera*, 2> cameras{{&topCamera_, &bottomCamera_}};
      if (!NaoCamera::waitForCameras(cameras, 200))
      {
        throw std::runtime_error("Something went wrong while trying to capture an image");
      }

      // If the images are older than the last used ones, get new images
      if (topCamera_.isImageValid() && topCamera_.getTimeStamp() < lastImageDataProduction_)
      {
        Log<M_TUHHSDK>(LogLevel::WARNING) << "Discarding image for TOP";
        topCamera_.releaseImage();
      }
      if (bottomCamera_.isImageValid() && bottomCamera_.getTimeStamp() < lastImageDataProduction_)
      {
        Log<M_TUHHSDK>(LogLevel::WARNING) << "Discarding image for BOTTOM";
        bottomCamera_.releaseImage();
      }
      allImagesAvailable = topCamera_.isImageValid() && bottomCamera_.isImageValid();
    } while (!allImagesAvailable);
  }


  // Use the older of the two images first
  if (topCamera_.isImageValid() &&
      (!bottomCamera_.isImageValid() || topCamera_.getTimeStamp() < bottomCamera_.getTimeStamp()))
  {
    currentCamera_ = CameraPosition::TOP;
    topCamera_.produce(cycleInfo, imageData);
  }
  else
  {
    currentCamera_ = CameraPosition::BOTTOM;
    bottomCamera_.produce(cycleInfo, imageData);
  }

  // Check for the correct timeline
  assert(cycleInfo.startTime >= lastImageDataProduction_);

  cycleInfo.cycleTime = cycleInfo.startTime - lastImageDataProduction_;
  cycleInfo.valid = true;
  lastImageDataProduction_ = cycleInfo.startTime;
}

std::string NaoInterface::getFileRoot() const
{
  return "/home/nao/naoqi/";
}

std::string NaoInterface::getDataRoot() const
{
  return getDataRootAndUSBStickState().first;
}

bool NaoInterface::isUSBStickMounted() const
{
  return getDataRootAndUSBStickState().second;
}

const RobotInfo& NaoInterface::getRobotInfo()
{
  return robotInfo_;
}

const RobotMetrics& NaoInterface::getRobotMetrics()
{
  return robotMetrics_;
}

AudioInterface& NaoInterface::getAudio()
{
  return audioInterface_;
}

FakeDataInterface& NaoInterface::getFakeData()
{
  return fakeData_;
}

void NaoInterface::initializeRobotConfiguration(Configuration& config)
{
  Log<M_TUHHSDK>(LogLevel::INFO) << "initializing RobotInfo";

  const auto bodyID{std::string{reinterpret_cast<char*>(robotConfiguration_.bodyId.data()),
                                robotConfiguration_.bodyId.size()}};
  const auto headID{std::string{reinterpret_cast<char*>(robotConfiguration_.headId.data()),
                                robotConfiguration_.headId.size()}};

  Log<M_TUHHSDK>(LogLevel::INFO) << "Body ID: " << bodyID;
  Log<M_TUHHSDK>(LogLevel::INFO) << "Head ID: " << headID;

  config.mount("NaoInterface.id_map", "id_map.json", ConfigurationType::HEAD);
  Uni::Value& val = config.get("NaoInterface.id_map", "idmap.nao");

  std::string bodyname;
  std::string headname;
  try
  {
    for (auto it = val.vectorBegin(); it != val.vectorEnd(); it++)
    {
      const auto& bid = (*it)["bodyid"].asString();
      const auto& hid = (*it)["headid"].asString();

      if (bid == bodyID)
      {
        bodyname = (*it)["name"].asString();
      }

      if (hid == headID)
      {
        headname = (*it)["name"].asString();
      }
    }
  }
  catch (std::exception& err)
  {
    Log<M_TUHHSDK>(LogLevel::ERROR)
        << "Error when finding out my identity (NaoInterface::initRobotInfo): " << err.what();
    throw std::runtime_error("Error when finding out my identity");
  }

  if (bodyname.empty())
  {
    Log<M_TUHHSDK>(LogLevel::ERROR) << "body could not be identified";
    throw std::runtime_error("Could not determine body name");
  }
  if (headname.empty())
  {
    Log<M_TUHHSDK>(LogLevel::ERROR) << "head could not be identified";
    throw std::runtime_error("Could not determine head name");
  }

  robotInfo_.bodyName = bodyname;
  robotInfo_.headName = headname;

  // Determine version
  if (robotConfiguration_.bodyVersion == 6)
  {
    robotInfo_.bodyVersion = RobotVersion::V6;
  }
  else
  {
    robotInfo_.bodyVersion = RobotVersion::UNKNOWN;
  }

  if (robotConfiguration_.headVersion == 6)
  {
    robotInfo_.headVersion = RobotVersion::V6;
  }
  else
  {
    robotInfo_.headVersion = RobotVersion::UNKNOWN;
  }

  // Export the NaoInfo to provide it in tuhhSDK.base for Export Diff functionality in MATE
  Uni::Value value = Uni::Value(Uni::ValueType::OBJECT);
  value << robotInfo_;
  config.set("tuhhSDK.base", "RobotInfo", value);
}
