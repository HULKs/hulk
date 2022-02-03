#include <ctime>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <thread>

#include "Data/ReplayData.hpp"
#include "Framework/Configuration/Configuration.h"
#include "Framework/Debug/Debug.h"
#include "Libs/json/json.h"
#include "Tools/Storage/UniValue/UniValue2JsonString.h"

#include "Framework/Debug/PngConverter.h"
#include "Framework/Log/Log.hpp"

#include "Framework/Debug/FileTransport.h"

class FileTransport::Impl
{
public:
  Impl(Debug& debug, Configuration& cfg, const std::string& fileRoot);
  Impl(const Impl&) = delete;
  Impl(const Impl&&) = delete;
  Impl operator=(const Impl&) = delete;
  Impl operator=(const Impl&&) = delete;
  ~Impl();

  void init();

  void transport();
  void updateGameControllerState();

  PngConverter img_conv_;
  Debug& debug_;
  Configuration& config_;

  uint64_t cycles_;
  std::string current_log_dir_;

  /// a list if subscribed keys
  std::set<std::string> subscriptionList_;

  /// the filestream for the replay.json file
  std::ofstream frameStream_;

  PngConverter pngConverter_;
  CVData compressedImage_;

  bool initDone_;
  bool firstFrame_;
  bool onlyRecordWhilePlaying_;
  bool gameStateIsPenalizedOrFinished_;
};

FileTransport::Impl::Impl(Debug& debug, Configuration& cfg, const std::string& fileRoot)
  : debug_(debug)
  , config_(cfg)
  , cycles_(0uL)
  , initDone_(false)
  , firstFrame_(true)
  , onlyRecordWhilePlaying_(true)
  , gameStateIsPenalizedOrFinished_(true)
{
  // Manage configurable parameters
  const std::string mount = "tuhhSDK.fileTransport";
  cfg.mount(mount, "fileTransport.json", ConfigurationType::HEAD);

  onlyRecordWhilePlaying_ = cfg.get(mount, "onlyRecordWhilePlaying").asBool();

  // Subscribe all configured keys
  Uni::Value& v = cfg.get(mount, "subscribedKeys");
  assert(v.type() == Uni::ValueType::ARRAY);
  for (auto it = v.vectorBegin(); it != v.vectorEnd(); ++it)
  {
    subscriptionList_.insert(it->asString());
    debug_.subscribe(it->asString());
  }
  debug_.subscribe("GameController.penalizedOrFinished");

  // Initialize
  std::stringstream ss;
  ss << fileRoot;
  std::time_t t = std::time(nullptr);
  ss << "filetransport"
     << "_" << std::put_time(std::gmtime(&t), "%Y-%m-%d_%H-%M-%S");
  ss << "/";
  current_log_dir_ = ss.str();
  std::filesystem::create_directory(current_log_dir_);
  frameStream_.open(current_log_dir_ + "/replay.json", std::ios_base::out | std::ios_base::trunc);
}

FileTransport::Impl::~Impl()
{
  frameStream_ << "]}" << std::endl;
  frameStream_.close();
}

void FileTransport::Impl::init()
{
  auto configMounts = config_.getMountPoints();
  std::vector<ReplayConfig> configs;
  for (auto& entry : configMounts)
  {
    auto& mount = entry.first;
    for (auto& key : config_.getKeyList(mount))
    {
      auto& configData = config_.get(mount, key);
      ReplayConfig config;
      config.mount = mount;
      config.key = key;
      config.data = configData;
      configs.push_back(config);
    }
  }

  Uni::Value exportConfig;
  exportConfig << configs;
  const std::string configString = Uni::Converter::toJsonString(exportConfig, false);
  frameStream_ << "{ \"config\":" << configString << "," << std::endl;
  frameStream_ << "\"frames\": [" << std::endl;
  initDone_ = true;
}

void FileTransport::Impl::transport()
{
  if (!initDone_)
  {
    init();
  }

  updateGameControllerState();
  if (!onlyRecordWhilePlaying_ && !gameStateIsPenalizedOrFinished_)
  {
    return;
  }

  if (firstFrame_)
  {
    frameStream_ << "[";
    firstFrame_ = false;
  }
  else
  {
    frameStream_ << ",[";
  }

  const auto& debugSources = debug_.getDebugSources();
  bool isFirst = true;

  for (const auto& key : subscriptionList_)
  {
    const DebugDatabase::DebugMapEntry* debugMapEntry = nullptr;
    for (const auto& debugSource : debugSources)
    {
      const auto currentDebugMap = debugSource.second.currentDebugMap;
      if (currentDebugMap != nullptr)
      {
        const auto it = currentDebugMap->getDebugMap().find(key);
        if (it != currentDebugMap->getDebugMap().end() &&
            currentDebugMap->getUpdateTime() == it->second.updateTime)
        {
          debugMapEntry = &(it->second);
          break;
        }
      }
    }

    // skip debug map entries that were not found
    if (debugMapEntry == nullptr)
    {
      Log<M_TUHHSDK>(LogLevel::DEBUG) << "Key might only be available in another debugSource!";
      continue;
    }

    // skip empty debug map entries
    if (debugMapEntry->data->type() == Uni::ValueType::NIL &&
        debugMapEntry->image->size == Vector2i::Zero())
    {
      continue;
    }

    Uni::Value debugDataToWrite;

    if (debugMapEntry->isImage)
    {
      std::string fileName;
      {
        std::stringstream fN;
        fN << current_log_dir_ << "/" << key << "_";
        fN << cycles_ << ".png";
        fileName = fN.str();
      }
      Uni::Value fileNameUniValue(fileName);
      const DebugData debugData(key, &fileNameUniValue, debugMapEntry->updateTime);
      debugDataToWrite << debugData;

      // Write image
      std::fstream file(fileName, std::ios::binary | std::ios::out | std::ios::trunc);
      pngConverter_.convert(*debugMapEntry->image, compressedImage_);
      file.write(reinterpret_cast<char*>(compressedImage_.data()), compressedImage_.size());
      file.close();
    }
    else
    {
      const DebugData debugData(key, debugMapEntry->data.get(), debugMapEntry->updateTime);
      debugDataToWrite << debugData;
    }

    const std::string json = Uni::Converter::toJsonString(debugDataToWrite, false);
    if (!isFirst)
    {
      frameStream_ << ",";
    }
    isFirst = false;
    frameStream_ << json;
  }

  cycles_++;
  frameStream_ << "]";
}

void FileTransport::Impl::updateGameControllerState()
{
  const auto& debugSources = debug_.getDebugSources();

  for (const auto& debugSource : debugSources)
  {
    auto currentDebugMap = debugSource.second.currentDebugMap;
    if (currentDebugMap != nullptr)
    {
      const auto it = currentDebugMap->getDebugMap().find("GameController.penalizedOrFinished");
      if (it != currentDebugMap->getDebugMap().end())
      {
        gameStateIsPenalizedOrFinished_ = it->second.data->asBool();
        break;
      }
    }
  }
}

FileTransport::FileTransport(Debug& debug, Configuration& cfg, const std::string& filePath)
  : pimpl_(new Impl(debug, cfg, filePath))
{
}

FileTransport::~FileTransport() {}

void FileTransport::transport()
{
  pimpl_->transport();
}
