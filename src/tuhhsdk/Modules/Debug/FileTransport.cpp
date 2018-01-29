#include <ctime>
#include <fstream>
#include <iomanip>
#include <thread>

#include <boost/filesystem.hpp>

#include "Libs/json/json.h"
#include "Modules/Configuration/Configuration.h"
#include "Modules/Debug/Debug.h"
#include "Tools/Storage/UniValue/UniValue2JsonString.h"
#include "Tools/Var/SpscQueue.hpp"

#include "PngConverter.h"
#include "StreamCompression.h"

#include "FileTransport.h"


#define MEMORY_FOR_DEBUGDATA 10 * 1024 * 1024

// Class implementations

struct ImageContainer
{
  std::shared_ptr<Image> image_;
  std::string filename_;
};

// Type definitions
typedef SpscRing<Uni::Value, 256> DebugDataRing;
typedef SpscRing<ImageContainer, 256> DebugImageRing;
typedef std::unordered_map<std::string, DebugData> DebugDataMap;
typedef std::unordered_map<std::string, TimePoint> ImageTimes;

class FileTransport::Impl
{
public:
  Impl(Debug& debug, Configuration& cfg, const std::string& fileRoot);

  ~Impl();

  void updateImageExportFrequency(const Uni::Value& value);

  void updateThreadRunFrequency(const Uni::Value& value);

  void writeDebugData();

  void writeImageData();

  void restartStreamCompression();

  void run_thread();

  void write_thread();

  int image_export_frequency_;
  int thread_run_frequency_;

  // Thread specific stuff
  std::thread writer;
  bool stop_thread = false;
  bool write_data = false;
  TimePoint last_write_;

  // Thread safe data
  DebugDataRing debug_ring_;
  DebugImageRing image_ring_;

  // Cycle data
  DebugDataMap debug_data_;
  ImageTimes image_times_;

  PngConverter img_conv_;
  StreamCompression stream_comp_;

  uint64_t cycles_;
  std::string current_log_dir_;
};

FileTransport::Impl::Impl(Debug& debug, Configuration& cfg, const std::string& fileRoot)
  : stream_comp_(MEMORY_FOR_DEBUGDATA)
  , cycles_(0)
{
  // Manage configurable parameters
  const std::string mount = "tuhhSDK.fileTransport";
  cfg.mount(mount, "fileTransport.json", ConfigurationType::HEAD);

  cfg.registerCallback(mount, "imageExportFrequency_", boost::bind(&FileTransport::Impl::updateImageExportFrequency, this, _1));
  cfg.registerCallback(mount, "threadRunFrequency_", boost::bind(&FileTransport::Impl::updateThreadRunFrequency, this, _1));

  image_export_frequency_ = cfg.get(mount, "imageExportFrequency_").asInt();
  thread_run_frequency_ = cfg.get(mount, "threadRunFrequency_").asInt();

  // Subscribe all configured keys
  Uni::Value& v = cfg.get(mount, "subscribedKeys");
  assert(v.type() == Uni::ValueType::ARRAY);
  for (auto it = v.listBegin(); it != v.listEnd(); ++it)
  {
    debug.subscribe(it->asString());
  }
  debug.subscribe("GameController.penalizedOrFinished");

  // Initialize
  std::stringstream ss;
  ss << fileRoot;
  std::time_t t = std::time(nullptr);
  ss << "filetransport"
     << "_" << std::put_time(std::gmtime(&t), "%Y-%m-%d_%H-%M-%S");
  ss << "/";
  current_log_dir_ = ss.str();
  stream_comp_.setFolder(current_log_dir_ + "data");
  boost::filesystem::create_directory(current_log_dir_);

  last_write_ = TimePoint::getCurrentTime();
  run_thread();
}

FileTransport::Impl::~Impl()
{
  stop_thread = true;
  if (writer.joinable())
  {
    writer.join();
  }
}

void FileTransport::Impl::run_thread()
{
  writer = std::thread(&Impl::write_thread, this);
}

void FileTransport::Impl::write_thread()
{
  while (!stop_thread)
  {
    writeDebugData();
    writeImageData();

    if (write_data && getTimeDiff(TimePoint::getCurrentTime(), last_write_, TDT::SECS) > 30)
    {
      restartStreamCompression();

      last_write_ = TimePoint::getCurrentTime();
      write_data = false;
    }

    std::this_thread::sleep_for(std::chrono::milliseconds(thread_run_frequency_));
  }
}

void FileTransport::Impl::updateImageExportFrequency(const Uni::Value& value)
{
  image_export_frequency_ = value.asInt();
}

void FileTransport::Impl::updateThreadRunFrequency(const Uni::Value& value)
{
  thread_run_frequency_ = value.asInt();
}

FileTransport::FileTransport(Debug& debug, Configuration& cfg, const std::string& filePath)
  : pimpl_(new Impl(debug, cfg, filePath))
{
  pimpl_->stream_comp_.openStream();
}

FileTransport::~FileTransport()
{
  pimpl_->stream_comp_.endStream();
}

void FileTransport::update(const DebugData& data)
{
  pimpl_->debug_data_[data.key] = data;
}

void FileTransport::sendImage(const std::string& key, const Image& img)
{
  auto penalizedOrFinished = pimpl_->debug_data_.find("GameController.penalizedOrFinished");
  if (penalizedOrFinished == pimpl_->debug_data_.end())
  {
    return;
  }
  const Uni::Value& v = penalizedOrFinished->second.value;
  if (v.type() != Uni::ValueType::BOOL || v.asBool())
  {
    return;
  }
  auto it = pimpl_->image_times_.find(key);
  if (it == pimpl_->image_times_.end() || getTimeDiff(TimePoint::getCurrentTime(), it->second, TDT::MILS) > pimpl_->image_export_frequency_)
  {
    std::stringstream ss;
    ImageContainer imgCont;

    ss << key << "_" << pimpl_->cycles_ << ".png";

    imgCont.image_ = std::make_shared<Image>(Image(img));
    imgCont.filename_ = ss.str();

    auto data = DebugData(key, Uni::Value(imgCont.filename_));

    pimpl_->image_times_[key] = TimePoint::getCurrentTime();
    pimpl_->debug_data_[key] = data;
    pimpl_->image_ring_.push(imgCont);
  }
}

void FileTransport::pushQueue(const std::string& /*key*/, const std::string& /*message*/) {}

void FileTransport::transport()
{

  // Increment cycle
  pimpl_->cycles_++;

  // Send collected data to thread
  if (pimpl_->cycles_ % 10 == 0)
  {
    Uni::Value root(Uni::ValueType::ARRAY);
    int i = 0;
    for (auto it = pimpl_->debug_data_.begin(); it != pimpl_->debug_data_.end(); it++)
    {
      it->second.toValue(root[i++]);
    }

    pimpl_->debug_ring_.push(root);
  }

  auto penalizedOrFinished = pimpl_->debug_data_.find("GameController.penalizedOrFinished");
  if (penalizedOrFinished == pimpl_->debug_data_.end())
  {
    return;
  }
  const Uni::Value& v = penalizedOrFinished->second.value;
  if (v.type() == Uni::ValueType::BOOL && v.asBool())
  {
    pimpl_->write_data = true;
  }
}

void FileTransport::Impl::restartStreamCompression()
{
  stream_comp_.endStream();
  stream_comp_.openStream();
}

void FileTransport::Impl::writeDebugData()
{
  Uni::Value value;
  while (debug_ring_.pop(value))
  {
    std::stringstream ss;

    ss << "\""
       << "cycle_" << TimePoint::getCurrentTime().getSystemTime() << "\"";

    if (value.size() == 0)
    {
      return;
    }

    std::string json = Uni::Converter::toJsonString(value);

    stream_comp_.writeData(ss.str() + " : ");
    stream_comp_.writeData(json + ",");
  }
}

void FileTransport::Impl::writeImageData()
{
  ImageContainer img_cont;
  int i = 0;
  while (image_ring_.pop(img_cont))
  {
    std::ofstream fs;
    auto image = img_conv_.convert(*img_cont.image_);

    std::string fn = current_log_dir_ + img_cont.filename_;
    fs.open(fn, std::ios_base::out | std::ios_base::trunc | std::ios_base::binary);
    fs.write((const char*)image->data(), image->size());
    fs.close();

    if (i == 10 && !write_data)
    {
      break;
    }
    i++;
  }
}
