// Parts of this file are taken from Src/Controller/SimulatedRobot.cpp from BHumanCodeRelease 2016

#include "Hardware/SimRobot/SimRobotInterface.hpp"
#include "Framework/Log/Log.hpp"
#include "Framework/tuhh.hpp"
#include "Hardware/Definitions.hpp"
#include "Hardware/SimRobot/HULKsMenu.hpp"
#include "Hardware/SimRobot/SimRobotAdapter.hpp"
#include "Hardware/SimRobot/SimRobotNoAudio.hpp"
#include "Hardware/SimRobot/SimRobotPortAudio.hpp"
#include "Tools/Math/Eigen.hpp"
#include <QList>
#include <QString>
#include <QVector>
#include <chrono>
#include <cmath>
#include <mutex>
#include <thread>

SimRobotInterface::SimRobotInterface(SimRobot::Application& application, SimRobot::Object* robot)
  : application_{application}
  , robot_{robot}
  , topCamera_{CameraPosition::TOP}
  , bottomCamera_{CameraPosition::BOTTOM}
  , robotName_{robot_->getFullName().mid(robot_->getFullName().lastIndexOf('.') + 1).toStdString()}
{
  QVector<QString> parts{1};

  // joints
  QString position(".position");
  for (std::size_t i{0}; i < static_cast<std::size_t>(Joints::MAX); ++i)
  {
    const auto joint{static_cast<Joints>(i)};
    parts[0] = QString(JOINT_NAMES[joint]) + position;
    jointSensors_[joint] = reinterpret_cast<SimRobotCore2::SensorPort*>(
        application_.resolveObject(parts, robot_, SimRobotCore2::sensorPort));
    jointActuators_[joint] = reinterpret_cast<SimRobotCore2::ActuatorPort*>(
        application_.resolveObject(parts, robot_, SimRobotCore2::actuatorPort));
  }

  // feet
  parts[0] = "LFoot";
  leftFoot_ = reinterpret_cast<SimRobotCore2::Body*>(
      application_.resolveObject(parts, robot_, SimRobotCore2::body));
  parts[0] = "RFoot";
  rightFoot_ = reinterpret_cast<SimRobotCore2::Body*>(
      application_.resolveObject(parts, robot_, SimRobotCore2::body));

  // gyroscope
  parts[0] = "Gyroscope.angularVelocities";
  gyroscope_ = application_.resolveObject(parts, robot_, SimRobotCore2::sensorPort);

  // accelerometer
  parts[0] = "Accelerometer.acceleration";
  accelerometer_ = application_.resolveObject(parts, robot_, SimRobotCore2::sensorPort);

  // cameras
  parts[0] = "CameraTop.image";
  cameras_[0] = application_.resolveObject(parts, robot_, SimRobotCore2::sensorPort);
  {
    const QList<int>& dimensions{
        reinterpret_cast<SimRobotCore2::SensorPort*>(cameras_[0])->getDimensions()};
    assert(dimensions.size() == 3);
    assert(dimensions[2] == 3);
    topCamera_.setSize(dimensions[0], dimensions[1]);
  }

  parts[0] = "CameraBottom.image";
  cameras_[1] = application_.resolveObject(parts, robot_, SimRobotCore2::sensorPort);
  {
    const QList<int>& dimensions{
        reinterpret_cast<SimRobotCore2::SensorPort*>(cameras_[1])->getDimensions()};
    assert(dimensions.size() == 3);
    assert(dimensions[2] == 3);
    bottomCamera_.setSize(dimensions[0], dimensions[1]);
  }

  // ball
  const auto* const balls{dynamic_cast<SimRobotCore2::Object*>(
      application_.resolveObject("RoboCup.balls", SimRobotCore2::compound))};
  if (balls != nullptr)
  {
    ball_ = application_.getObjectChild(*balls, 0);
  }

  // other robots
  const auto* const group{application_.resolveObject("RoboCup.robots", SimRobotCore2::compound)};
  // at this point it is ensured that the RoboCup.robots object has at least one member
  // since otherwise this code would not be called
  const auto totalNumberOfRobots{application_.getObjectChildCount(*group)};
  assert(totalNumberOfRobots > 0);
  for (int i = 0; i < totalNumberOfRobots; i++)
  {
    auto* const robot{static_cast<SimRobot::Object*>(application_.getObjectChild(*group, i))};
    // make sure that we don't add ourselfs
    if (robot->getFullName() != robot_->getFullName())
    {
      otherRobots_.push_back(robot);
    }
  }

  // init tuhhSDK
  tuhh_ = std::make_unique<TUHH>(*this);
}

SimRobotInterface::~SimRobotInterface()
{
  topCamera_.setShutdownRequest();
  bottomCamera_.setShutdownRequest();
  shutdownRequested_.store(true);
  sensorDataConditionVariable_.notify_all();
  imagesRendered_.notify_all();
  // TUHH instance needs to be cleared before condition variables
  tuhh_.reset();
}

void SimRobotInterface::update(std::uint64_t simulatedSteps)
{
  lastTimePoint_ = currentTimePoint_;
  currentTimePoint_ =
      Clock::time_point{std::chrono::duration<Clock::rep, std::centi>{simulatedSteps}};

  // render camera images every third frame
  const auto renderImages{(counter_ % 3) == 0};
  if (renderImages)
  {
    if (topCamera_.isEnabled() || bottomCamera_.isEnabled())
    {
      std::unique_lock lock{cameraMutex_};
      if (SimRobotCamera::renderCameras({&topCamera_, &bottomCamera_}, cameras_, currentTimePoint_))
      {
        lock.unlock();
        imagesRendered_.notify_all();
      }
    }
    else
    {
      {
        std::unique_lock lock{cameraMutex_};
        // if no real image is requested, set an empty one to trigger the waiting thread
        topCamera_.setImage(nullptr, currentTimePoint_);
        bottomCamera_.setImage(nullptr, currentTimePoint_ + 1ms);
      }
      imagesRendered_.notify_all();
    }
  }

  {
    const auto jointAngles{[this] {
      std::unique_lock lock{jointAnglesMutex_};
      jointAnglesConditionVariable_.wait(lock, [this] { return jointAnglesAvailable_; });
      jointAnglesAvailable_ = false;
      return jointAngles_;
    }()};

    std::lock_guard lock{sensorDataMutex_};

    sensorDataCycleInfo_.startTime = currentTimePoint_;
    // sensorDataCycleInfo_.cycleTime will be set in produceSensorData()
    sensorDataCycleInfo_.valid = true;

    for (std::size_t i{0}; i < static_cast<std::size_t>(Joints::MAX); ++i)
    {
      const auto joint{static_cast<Joints>(i)};
      if (jointActuators_[joint] == nullptr)
      {
        sensorDataJointSensorData_.angles[joint] = 0.f;
      }
      else
      {
        sensorDataJointSensorData_.angles[joint] =
            dynamic_cast<SimRobotCore2::SensorPort*>(jointSensors_[joint])->getValue().floatValue;
        reinterpret_cast<SimRobotCore2::ActuatorPort*>(jointActuators_[joint])
            ->setValue(jointAngles[joint]);
      }
      sensorDataJointSensorData_.currents[joint] = 0.f;
      sensorDataJointSensorData_.temperatures[joint] = 30.f;
      sensorDataJointSensorData_.status[joint] = 0.f;
    }
    sensorDataJointSensorData_.valid = true;

    sensorDataButtonData_.switches = SwitchInfo{};
    bool expected{true};
    if (chestButtonWasRequested_.compare_exchange_weak(expected, false))
    {
      sensorDataButtonData_.switches.isChestButtonPressed = true;
    }
    bool singlePressDetected{!sensorDataButtonData_.switches.isChestButtonPressed &&
                             chestButtonWasPressedLastUpdate_};
    if (singlePressDetected)
    {
      sensorDataButtonData_.lastChestButtonSinglePress = currentTimePoint_;
    }
    sensorDataButtonData_.valid = true;
    chestButtonWasPressedLastUpdate_ = sensorDataButtonData_.switches.isChestButtonPressed;

    const float* floatArray{
        reinterpret_cast<SimRobotCore2::SensorPort*>(gyroscope_)->getValue().floatArray};
    sensorDataIMUSensorData_.gyroscope.x() = floatArray[0];
    sensorDataIMUSensorData_.gyroscope.y() = floatArray[1];
    sensorDataIMUSensorData_.gyroscope.z() = -floatArray[2];
    floatArray =
        reinterpret_cast<SimRobotCore2::SensorPort*>(accelerometer_)->getValue().floatArray;
    sensorDataIMUSensorData_.accelerometer.x() = -floatArray[0];
    sensorDataIMUSensorData_.accelerometer.y() = floatArray[1];
    sensorDataIMUSensorData_.accelerometer.z() = -floatArray[2];
    std::array<float, 3> position{};
    std::array<float, 9> world2Robot{};
    reinterpret_cast<SimRobotCore2::Body*>(robot_)->getPose(
        // NOLINTNEXTLINE(hicpp-avoid-c-arrays,modernize-avoid-c-arrays,cppcoreguidelines-avoid-c-arrays)
        position.data(), reinterpret_cast<float(*)[3]>(world2Robot.data()));
    const Vector2f axis{world2Robot[0 * 3 + 5], -world2Robot[2]};
    const auto axisLength{std::sqrt(axis.x() * axis.x() + axis.y() * axis.y())};
    if (axisLength == 0.f)
    {
      sensorDataIMUSensorData_.angle.x() = 0.f;
      sensorDataIMUSensorData_.angle.y() = 0.f;
    }
    else
    {
      const auto w{std::atan2(axisLength, world2Robot[2 * 3 + 2]) / axisLength};
      sensorDataIMUSensorData_.angle.x() = axis.x() * w;
      sensorDataIMUSensorData_.angle.y() = axis.y() * w;
    }
    sensorDataIMUSensorData_.valid = true;

    // Fake data:
    // the faked position of this robot:
    Pose robotPose{position[0], position[1],
                   std::atan2(-world2Robot[1 * 3 + 0], world2Robot[0 * 3 + 0])};
    fakeData_.setFakeRobotPose(robotPose);
    const auto inverseRobotPose{robotPose.inverse()};
    if (ball_ != nullptr)
    {
      const auto* const absBall{reinterpret_cast<SimRobotCore2::Body*>(ball_)->getPosition()};
      const Vector2f absoluteBallPosition{absBall[0], absBall[1]};
      const Vector2f relativeBallPosition{inverseRobotPose * absoluteBallPosition};
      fakeData_.setFakeBallPosition(relativeBallPosition);
    }
    // the faked position of other robots
    // extract the postions from the otherRobots_
    VecVector2f otherRobotPositions;
    otherRobotPositions.reserve(otherRobots_.size());
    for (auto& otherRobot : otherRobots_)
    {
      std::array<float, 3> position{};
      std::array<float, 9> world2Robot{};
      reinterpret_cast<SimRobotCore2::Body*>(otherRobot)
          // NOLINTNEXTLINE(hicpp-avoid-c-arrays,modernize-avoid-c-arrays,cppcoreguidelines-avoid-c-arrays)
          ->getPose(position.data(), reinterpret_cast<float(*)[3]>(world2Robot.data()));
      const auto relativeOtherRobot{inverseRobotPose * Vector2f{position[0], position[1]}};
      otherRobotPositions.emplace_back(relativeOtherRobot);
    }
    fakeData_.setFakeRobotPositions(otherRobotPositions);
    updateFSRs();
    sensorDataSonarSensorData_.data = SonarInfo{};
    sensorDataSonarSensorData_.valid[Sonars::LEFT] = false;
    sensorDataSonarSensorData_.valid[Sonars::RIGHT] = false;

    sensorDataAvailable_ = true;
  }
  sensorDataConditionVariable_.notify_all();

  counter_++;
}

void SimRobotInterface::configure(Configuration& config)
{
  robotInfo_.bodyVersion = RobotVersion::V6;
  robotInfo_.headVersion = RobotVersion::V6;
  robotInfo_.bodyName = robotName_;
  robotInfo_.headName = robotName_;
  config.setNaoHeadName(robotInfo_.headName);
  config.setNaoBodyName(robotInfo_.bodyName);
  // Export the NaoInfo to provide it in tuhhSDK.base for Export Diff functionality in MATE
  // (Not really applicable for SimRobot?)
  Uni::Value value{Uni::ValueType::OBJECT};
  value << robotInfo_;
  config.set("tuhhSDK.base", "RobotInfo", value);
  std::string mount{"SimRobot"};
  config.mount(mount, mount + ".json", ConfigurationType::HEAD);

  if (config.get(mount, "enablePortaudio").asBool())
  {
    audio_ = std::make_unique<SimRobotPortAudio>();
  }
  else
  {
    audio_ = std::make_unique<SimRobotNoAudio>();
  }

  robotMetrics_.configure(config, robotInfo_);
}

void SimRobotInterface::setJointAngles(const JointsArray<float>& angles)
{
  {
    std::lock_guard lock{jointAnglesMutex_};
    jointAngles_ = angles;
    jointAnglesAvailable_ = true;
  }
  jointAnglesConditionVariable_.notify_all();
}

void SimRobotInterface::setJointStiffnesses(const JointsArray<float>& /*stiffnesses*/) {}

void SimRobotInterface::setLEDs(const Led::Chest& /*chest*/, const Led::Ear& /*leftEar*/,
                                const Led::Ear& /*righEar*/, const Led::Eye& /*leftEye*/,
                                const Led::Eye& /*rightEye*/, const Led::Foot& /*leftFoot*/,
                                const Led::Foot& /*rightFoot*/)
{
}

std::string SimRobotInterface::getFileRoot() const
{
  return LOCAL_FILE_ROOT;
}

std::string SimRobotInterface::getDataRoot() const
{
  return getFileRoot();
}

const RobotInfo& SimRobotInterface::getRobotInfo()
{
  return robotInfo_;
}

const RobotMetrics& SimRobotInterface::getRobotMetrics()
{
  return robotMetrics_;
}

FakeDataInterface& SimRobotInterface::getFakeData()
{
  return fakeData_;
}

AudioInterface& SimRobotInterface::getAudio()
{
  return *audio_;
}

void SimRobotInterface::pressChestButton()
{
  chestButtonWasRequested_.store(true);
}

const std::string& SimRobotInterface::getName() const
{
  return robotName_;
}

void SimRobotInterface::produceSensorData(CycleInfo& cycleInfo, FSRSensorData& fsrSensorData,
                                          IMUSensorData& imuSensorData,
                                          JointSensorData& jointSensorData, ButtonData& buttonData,
                                          SonarSensorData& sonarSensorData)
{
  std::unique_lock lock{sensorDataMutex_};
  sensorDataConditionVariable_.wait(
      lock, [this] { return sensorDataAvailable_ || shutdownRequested_.load(); });
  sensorDataAvailable_ = false;
  cycleInfo = sensorDataCycleInfo_;
  cycleInfo.cycleTime = cycleInfo.startTime - lastSensorDataProduction_;
  cycleInfo.valid = true;
  lastSensorDataProduction_ = cycleInfo.startTime;
  fsrSensorData = sensorDataFSRSensorData_;
  imuSensorData = sensorDataIMUSensorData_;
  jointSensorData = sensorDataJointSensorData_;
  buttonData = sensorDataButtonData_;
  sonarSensorData = sensorDataSonarSensorData_;
}

void SimRobotInterface::enableImageDataProducer()
{
  topCamera_.enable();
  bottomCamera_.enable();
}

void SimRobotInterface::disableImageDataProducer()
{
  topCamera_.disable();
  bottomCamera_.disable();
}

void SimRobotInterface::produceImageData(CycleInfo& cycleInfo, ImageData& imageData)
{
  std::unique_lock lock{cameraMutex_};
  std::array<SimRobotCamera*, 2> cameras{&topCamera_, &bottomCamera_};
  imagesRendered_.wait(lock,
                       [&cameras] { return SimRobotCamera::getNextCamera(cameras) != nullptr; });
  auto* const camera{SimRobotCamera::getNextCamera(cameras)};
  camera->produce(cycleInfo, imageData);
  cycleInfo.cycleTime = cycleInfo.startTime - lastImageDataProduction_;
  lastImageDataProduction_ = cycleInfo.startTime;
  cycleInfo.valid = true;
}

void SimRobotInterface::updateFSRs()
{
  // the number of fsrs in each foot
  static constexpr auto fsrsPerFoot__{static_cast<unsigned int>(FSRs::MAX) / 2u};
  if ((leftFoot_ == nullptr) || (rightFoot_ == nullptr))
  {
    sensorDataFSRSensorData_.leftFoot.frontLeft = 0.5f;
    sensorDataFSRSensorData_.leftFoot.frontRight = 0.5f;
    sensorDataFSRSensorData_.leftFoot.rearLeft = 0.5f;
    sensorDataFSRSensorData_.leftFoot.rearRight = 0.5f;
    sensorDataFSRSensorData_.rightFoot.frontLeft = 0.5f;
    sensorDataFSRSensorData_.rightFoot.frontRight = 0.5f;
    sensorDataFSRSensorData_.rightFoot.rearLeft = 0.5f;
    sensorDataFSRSensorData_.rightFoot.rearRight = 0.5f;
  }
  // the positions of all fsrs (in m with respect to the foot center)
  FSRsArray<Vector2f> fsrPositions{
      {robotMetrics_.fsrPosition(FSRs::L_FRONT_LEFT),
       robotMetrics_.fsrPosition(FSRs::L_FRONT_RIGHT), robotMetrics_.fsrPosition(FSRs::L_REAR_LEFT),
       robotMetrics_.fsrPosition(FSRs::L_REAR_RIGHT), robotMetrics_.fsrPosition(FSRs::R_FRONT_LEFT),
       robotMetrics_.fsrPosition(FSRs::R_FRONT_RIGHT), robotMetrics_.fsrPosition(FSRs::R_REAR_LEFT),
       robotMetrics_.fsrPosition(FSRs::R_REAR_RIGHT)}};
  static constexpr auto weight__{0.415f};
  // figuring out how much weight we have on each foot
  FSRsArray<float> fsrData;
  for (std::size_t i = 0; i < static_cast<std::size_t>(FSRs::MAX); i++)
  {
    // in which foot is the current fsr-index located?
    const auto isLeftFootFsr{i < fsrsPerFoot__};
    // get the kinematic matrix of the foot, the current fsr-index belongs to
    KinematicMatrix foot2Ground{getKinematicMatrix(isLeftFootFsr ? leftFoot_ : rightFoot_)};
    // get the position of this fsr with respect to the foot center...
    const Vector2f& fsrToFoot{fsrPositions[static_cast<FSRs>(i)]};
    // ... figure out where it is in the world
    Vector3f fsrToGround{foot2Ground * Vector3f(fsrToFoot.x(), fsrToFoot.y(),
                                                -robotMetrics_.link(Links::FOOT_HEIGHT))};
    // reverse-engineer the fsr-reading from the amount the robot sank into the ground
    fsrData[static_cast<FSRs>(i)] = std::min(std::max(0.f, -fsrToGround.z() * weight__), 2.f);
  }
  sensorDataFSRSensorData_.leftFoot.frontLeft = fsrData[FSRs::L_FRONT_LEFT];
  sensorDataFSRSensorData_.leftFoot.frontRight = fsrData[FSRs::L_FRONT_RIGHT];
  sensorDataFSRSensorData_.leftFoot.rearLeft = fsrData[FSRs::L_REAR_LEFT];
  sensorDataFSRSensorData_.leftFoot.rearRight = fsrData[FSRs::L_REAR_RIGHT];
  sensorDataFSRSensorData_.rightFoot.frontLeft = fsrData[FSRs::R_FRONT_LEFT];
  sensorDataFSRSensorData_.rightFoot.frontRight = fsrData[FSRs::R_FRONT_RIGHT];
  sensorDataFSRSensorData_.rightFoot.rearLeft = fsrData[FSRs::R_REAR_LEFT];
  sensorDataFSRSensorData_.rightFoot.rearRight = fsrData[FSRs::R_REAR_RIGHT];
  sensorDataFSRSensorData_.leftFoot = sensorDataFSRSensorData_.leftFoot;
  sensorDataFSRSensorData_.totalLeft =
      sensorDataFSRSensorData_.leftFoot.frontLeft + sensorDataFSRSensorData_.leftFoot.frontRight +
      sensorDataFSRSensorData_.leftFoot.rearLeft + sensorDataFSRSensorData_.leftFoot.rearRight;
  sensorDataFSRSensorData_.rightFoot = sensorDataFSRSensorData_.rightFoot;
  sensorDataFSRSensorData_.totalRight =
      sensorDataFSRSensorData_.rightFoot.frontLeft + sensorDataFSRSensorData_.rightFoot.frontRight +
      sensorDataFSRSensorData_.rightFoot.rearLeft + sensorDataFSRSensorData_.rightFoot.rearRight;
  sensorDataFSRSensorData_.valid = true;
}

KinematicMatrix SimRobotInterface::getKinematicMatrix(SimRobot::Object* object)
{
  std::array<float, 3> position{};
  std::array<float, 9> rotation{};
  reinterpret_cast<SimRobotCore2::Body*>(object)
      // NOLINTNEXTLINE(hicpp-avoid-c-arrays,modernize-avoid-c-arrays,cppcoreguidelines-avoid-c-arrays)
      ->getPose(position.data(), reinterpret_cast<float(*)[3]>(rotation.data()));
  KinematicMatrix target;
  target.posV.x() = position[0];
  target.posV.y() = position[1];
  target.posV.z() = position[2];

  target.posV *= 1000.f;

  Matrix3f rot;
  rot << rotation[0 * 3 + 0], rotation[1 * 3 + 0], rotation[2 * 3 + 0], rotation[0 * 3 + 1],
      rotation[1 * 3 + 1], rotation[2 * 3 + 1], rotation[0 * 3 + 2], rotation[1 * 3 + 2],
      rotation[2 * 3 + 2];
  target.rotM = rot;
  return target;
}
