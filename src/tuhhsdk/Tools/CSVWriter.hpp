#pragma once

#include <boost/filesystem.hpp>
#include <fstream>

/**
 * @brief Utility class for writing CSV - Files.
 * NOTE: This is a very simple implementation. It only does the file handling. There is no logic like escaping!
 * @author Georg Felbinger
 */
template <size_t COLUMNS>
class CSVWriter
{
public:
  /**
   * @brief Constructor.
   * @param file the file to write.
   */
  CSVWriter(const std::string file)
    : file_(file)
  {
    assert(COLUMNS > 0);
    boost::filesystem::path p(file);
    boost::filesystem::create_directory(p.parent_path());
  }

  /**
   * @brief writes a line into the file.
   * @param data the strings (one for each column) to write.
   * @append wether the file should be appended or truncated (usefull for header lines).
   */
  void write(const std::array<std::string, COLUMNS>& data, const bool append = true) const
  {
    assert(data.size() == COLUMNS);
    std::ofstream dataStream;
    const auto openOption =
        std::ios_base::out | (append ? std::ios_base::app : std::ios_base::trunc);
    dataStream.open(file_, openOption);
    dataStream << data[0];
    for (size_t i = 1; i < data.size(); i++)
    {
      dataStream << SEP << data[i];
    }
    dataStream << std::endl;
    dataStream.close();
  }

private:
  const std::string file_;
  const std::string SEP = "|";
};

