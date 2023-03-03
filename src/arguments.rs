use clap::Parser;
use console::style;
use goblin::Object;
use goblin::pe::data_directories::DataDirectories;
use goblin::pe::header::{DosHeader, CoffHeader};
use goblin::pe::optional_header::OptionalHeader;
use goblin::pe::section_table::SectionTable;
use goblin::pe::{
  export::Export,
  import::Import,
};
use goblin::strtab::Strtab;
use std::fs::File;
use std::io::Read;
use std::os::raw::c_void;
use std::os::windows::prelude::FromRawHandle;
use std::path::Path;
use std::{
  fs::read,
};
use comfy_table::{Table, Cell, Row, Color};
use reqwest::{
  blocking::ClientBuilder, Method
};
use serde::Deserialize;

mod config_options {
  pub const CONFIG_FILE: &str = "config.conf";
  pub const ENABLE_VT_SEARCH: &str = "enable_virustotal_search";
  pub const API_KEY: &str = "apikey";
}

#[derive(Debug, Clone, Default)]
pub struct CmdSettings {
  pub api_key: String,
  pub auto_search_vt: bool,
  pub file_hash: String,
}

impl CmdSettings {
  pub fn new(file_hash: String) -> Self {
    Self {
      api_key: String::new(),
      auto_search_vt: false,
      file_hash: file_hash,
    }
  }
}

#[derive(Deserialize, Debug, Default)]
#[allow(dead_code)]
pub struct VtJsonOutput {
  data: VtData,
}

#[derive(Deserialize, Debug, Default)]
#[allow(dead_code)]
pub struct VtData {
  attributes: VtAttributes,
}

#[derive(Deserialize, Debug, Default)]
#[allow(dead_code)]
pub struct VtAttributes {
  last_analysis_results: AnalysisResults,
}

#[derive(Deserialize, Debug, Default)]
#[allow(dead_code, non_snake_case)]
pub struct AnalysisResults {
  Bkav: Option<AVProvider>,
  Lionic: Option<AVProvider>,
  Elastic: Option<AVProvider>,
  DrWeb: Option<AVProvider>,
  #[serde(rename = "MicroWorld-eScan")]
  MicroWorld_eScan: Option<AVProvider>,
  CMC: Option<AVProvider>,
  #[serde(rename = "CAT-QuickHeal")]
  CAT_QuickHeal: Option<AVProvider>,
  McAfee: Option<AVProvider>,
  Cylance: Option<AVProvider>,
  VIPRE: Option<AVProvider>,
  Sangfor: Option<AVProvider>,
  K7AntiVirus: Option<AVProvider>,
  Alibaba: Option<AVProvider>,
  K7GW: Option<AVProvider>,
  CrowdStrike: Option<AVProvider>,
  Arcabit: Option<AVProvider>,
  BitDefenderTheta: Option<AVProvider>,
  VirIT: Option<AVProvider>,
  Cyren: Option<AVProvider>,
  SymantecMobileInsight: Option<AVProvider>,
  Symantec: Option<AVProvider>,
  tehtris: Option<AVProvider>,
  #[serde(rename = "ESET-NOD32")]
  ESET_NOD32: Option<AVProvider>,
  APEX: Option<AVProvider>,
  Paloalto: Option<AVProvider>,
  ClamAV: Option<AVProvider>,
  Kaspersky: Option<AVProvider>,
  BitDefender: Option<AVProvider>,
  #[serde(rename = "NANO-Antivirus")]
  NANO_Antivirus: Option<AVProvider>,
  SUPERAntiSpyware: Option<AVProvider>,
  Tencent: Option<AVProvider>,
  Trustlook: Option<AVProvider>,
  TACHYON: Option<AVProvider>,
  #[serde(rename = "F-Secure")]
  F_Secure: Option<AVProvider>,
  Baidu: Option<AVProvider>,
  Zillya: Option<AVProvider>,
  TrendMicro: Option<AVProvider>,
  #[serde(rename = "McAfee-GW-Edition")]
  McAfee_GW_Edition: Option<AVProvider>,
  Trapmine: Option<AVProvider>,
  FireEye: Option<AVProvider>,
  Sophos: Option<AVProvider>,
  SentinelOne: Option<AVProvider>,
  #[serde(rename = "Avast-Mobile")]
  Avast_Mobile: Option<AVProvider>,
  Jiangmin: Option<AVProvider>,
  Webroot: Option<AVProvider>,
  Avira: Option<AVProvider>,
  #[serde(rename = "Antiy-AVL")]
  Antiy_AVL: Option<AVProvider>,
  Kingsoft: Option<AVProvider>,
  Gridinsoft: Option<AVProvider>,
  Xcitium: Option<AVProvider>,
  Microsoft: Option<AVProvider>,
  ViRobot: Option<AVProvider>,
  ZoneAlarm: Option<AVProvider>,
  GData: Option<AVProvider>,
  Google: Option<AVProvider>,
  BitDefenderFalx: Option<AVProvider>,
  #[serde(rename = "AhnLab-V3")]
  AhnLab_V3: Option<AVProvider>,
  Acronis: Option<AVProvider>,
  VBA32: Option<AVProvider>,
  ALYac: Option<AVProvider>,
  MAX: Option<AVProvider>,
  Panda: Option<AVProvider>,
  Zoner: Option<AVProvider>,
  #[serde(rename = "TrendMicro-HouseCall")]
  TrendMicro_HouseCall: Option<AVProvider>,
  Rising: Option<AVProvider>,
  Yandex: Option<AVProvider>,
  Ikarus: Option<AVProvider>,
  MaxSecure: Option<AVProvider>,
  Fortinet: Option<AVProvider>,
  AVG: Option<AVProvider>,
  Cybereason: Option<AVProvider>,
  Avast: Option<AVProvider>,
}

#[derive(Debug, Clone, Default)]
pub struct DataDirectoryInfo {
  pub name: String,
  pub size: u32,
  pub virtual_address: u32,
}

#[derive(Deserialize, Debug, Default)]
#[allow(dead_code)]
pub struct AVProvider {
  category: String,
  engine_name: String,
  engine_version: Option<String>,
  result: Option<String>,
  method: String,
  engine_update: String,
}

#[derive(Debug, Clone, Parser)]
#[clap(author, about, version)]
pub struct Arguments {
  #[clap(value_parser)]
  /// The file name or path
  pub filename: String,

  #[clap(short, long)]
  /// Manuallt enter apikey and query binary on virus total.
  pub api_key: Option<String>,

  #[clap(long, default_value_if("vt", Some("false"), Some("true")), min_values(0))]
  /// Shows a query on virus total. Must be enabled in the config file.
  pub vt: bool,
  
  #[clap(short, long, default_value_if("exports", Some("false"), Some("true")), min_values(0))]
  /// Exported functions.
  pub exports: bool,

  #[clap(long, default_value_if("ex_libs", Some("false"), Some("true")), min_values(0))]
  /// Shows libraies used by exported function.
  pub ex_libs: bool,
  
  #[clap(short, long, default_value_if("imports", Some("false"), Some("true")), min_values(0))]
  /// imported functions.
  pub imports: bool,

  #[clap(short, long, default_value_if("sections", Some("false"), Some("true")), min_values(0))]
  /// Sections
  pub sections: bool,

  #[clap(long = "Dh", default_value_if("Dh", Some("false"), Some("true")), min_values(0))]
  pub dos_header: bool,

  #[clap(long = "Oh", default_value_if("Oh", Some("false"), Some("true")), min_values(0))]
  pub optional_header: bool,
  
  #[clap(long = "Ch", default_value_if("Ch", Some("false"), Some("true")), min_values(0))]
  pub coff_header: bool,

  #[clap(short, long, default_value_if("directories", Some("false"), Some("true")), min_values(0))]
  pub directories: bool,
}

impl Arguments {

  /**Function unpacks the AnalysisResults struct into a vec so user does not have to manually extract data from 82 fields.
   * Params:
   *  d: AnalysisResult {The struct to unpack}
   * Returns Vec<AVProvider>
   */
  pub fn get_av_provider_data(d: AnalysisResults) -> Vec<AVProvider> {
    let mut out: Vec<AVProvider> = Default::default();

    if let Some(d) = d.Bkav                   { out.push(d); }
    if let Some(d) = d.Lionic                 { out.push(d); }
    if let Some(d) = d.Elastic                { out.push(d); }
    if let Some(d) = d.DrWeb                  { out.push(d); }
    if let Some(d) = d.MicroWorld_eScan       { out.push(d); }
    if let Some(d) = d.CMC                    { out.push(d); }
    if let Some(d) = d.CAT_QuickHeal          { out.push(d); }
    if let Some(d) = d.McAfee                 { out.push(d); }
    if let Some(d) = d.Cylance                { out.push(d); }
    if let Some(d) = d.VIPRE                  { out.push(d); }
    if let Some(d) = d.Sangfor                { out.push(d); }
    if let Some(d) = d.K7AntiVirus            { out.push(d); }
    if let Some(d) = d.Alibaba                { out.push(d); }
    if let Some(d) = d.K7GW                   { out.push(d); }
    if let Some(d) = d.CrowdStrike            { out.push(d); }
    if let Some(d) = d.Arcabit                { out.push(d); }
    if let Some(d) = d.BitDefenderTheta       { out.push(d); }
    if let Some(d) = d.VirIT                  { out.push(d); }
    if let Some(d) = d.Cyren                  { out.push(d); }
    if let Some(d) = d.SymantecMobileInsight  { out.push(d); }
    if let Some(d) = d.Symantec               { out.push(d); }
    if let Some(d) = d.tehtris                { out.push(d); }
    if let Some(d) = d.ESET_NOD32             { out.push(d); }
    if let Some(d) = d.APEX                   { out.push(d); }
    if let Some(d) = d.Paloalto               { out.push(d); }
    if let Some(d) = d.ClamAV                 { out.push(d); }
    if let Some(d) = d.Kaspersky              { out.push(d); }
    if let Some(d) = d.BitDefender            { out.push(d); }
    if let Some(d) = d.NANO_Antivirus         { out.push(d); }
    if let Some(d) = d.SUPERAntiSpyware       { out.push(d); }
    if let Some(d) = d.Tencent                { out.push(d); }
    if let Some(d) = d.Trustlook              { out.push(d); }
    if let Some(d) = d.TACHYON                { out.push(d); }
    if let Some(d) = d.F_Secure               { out.push(d); }
    if let Some(d) = d.Baidu                  { out.push(d); }
    if let Some(d) = d.Zillya                 { out.push(d); }
    if let Some(d) = d.TrendMicro             { out.push(d); }
    if let Some(d) = d.McAfee_GW_Edition      { out.push(d); }
    if let Some(d) = d.Trapmine               { out.push(d); }
    if let Some(d) = d.FireEye                { out.push(d); }
    if let Some(d) = d.Sophos                 { out.push(d); }
    if let Some(d) = d.SentinelOne            { out.push(d); }
    if let Some(d) = d.Avast_Mobile           { out.push(d); }
    if let Some(d) = d.Jiangmin               { out.push(d); }
    if let Some(d) = d.Webroot                { out.push(d); }
    if let Some(d) = d.Avira                  { out.push(d); }
    if let Some(d) = d.Antiy_AVL              { out.push(d); }
    if let Some(d) = d.Kingsoft               { out.push(d); }
    if let Some(d) = d.Gridinsoft             { out.push(d); }
    if let Some(d) = d.Xcitium                { out.push(d); }
    if let Some(d) = d.Microsoft              { out.push(d); }
    if let Some(d) = d.ViRobot                { out.push(d); }
    if let Some(d) = d.ZoneAlarm              { out.push(d); }
    if let Some(d) = d.GData                  { out.push(d); }
    if let Some(d) = d.Google                 { out.push(d); }
    if let Some(d) = d.BitDefenderFalx        { out.push(d); }
    if let Some(d) = d.AhnLab_V3              { out.push(d); }
    if let Some(d) = d.Acronis                { out.push(d); }
    if let Some(d) = d.VBA32                  { out.push(d); }
    if let Some(d) = d.ALYac                  { out.push(d); }
    if let Some(d) = d.MAX                    { out.push(d); }
    if let Some(d) = d.Panda                  { out.push(d); }
    if let Some(d) = d.Zoner                  { out.push(d); }
    if let Some(d) = d.TrendMicro_HouseCall   { out.push(d); }
    if let Some(d) = d.Rising                 { out.push(d); }
    if let Some(d) = d.Yandex                 { out.push(d); }
    if let Some(d) = d.Ikarus                 { out.push(d); }
    if let Some(d) = d.MaxSecure              { out.push(d); }
    if let Some(d) = d.Fortinet               { out.push(d); }
    if let Some(d) = d.AVG                    { out.push(d); }
    if let Some(d) = d.Cybereason             { out.push(d); } 
    if let Some(d) = d.Avast                  { out.push(d); }

    out
  }

  /**Function displays all data contained in the load file or just some of it in a table as specified by the user.
   * Params:
   *  &self
   *  bytes:         Vec<u8>          {The raw bytes of the loaded binary}
   *  settings:      &mut CmdSettings {The file hash and virus total search information}
   *  sh_everything: bool             {When true displays all information in the binary}
   * Returns Result<()>
   */
  pub fn display_data(&self, bytes: Vec<u8>, settings: &mut CmdSettings, sh_everything: bool) -> goblin::error::Result<()> {
    // let c_bytes = bytes.clone();

    match Object::parse(&bytes)? {
      Object::Elf(_) => {},
      
      Object::PE(pe) => {
        let imports = pe.imports;
        let exports = pe.exports;
        let sections = pe.sections;      
        let dos_header = pe.header.dos_header;
        let coff_header = pe.header.coff_header;
        let optional_header = pe.header.optional_header;
        // let string_table = coff_header.strings(&c_bytes);

        Self::load_config_file(settings).unwrap();
        if sh_everything == true {
          let export_table = Self::get_exports(&exports);
          println!("{export_table}");
          
          let export_lib_table = Self::get_exported_lib(exports);
          println!("{export_lib_table}");

          let import_table = Self::get_imports(&imports);
          println!("{import_table}");

          if self.vt == true {
            if settings.auto_search_vt.clone() == false {
              println!("{}: Virus Total search must be enabled in the config file", style("Error").red().bright());
            }

            else {
              println!("Querying [{}] on Virus Total", style(settings.file_hash.clone()).cyan());
              Self::search_virus_total(&settings.file_hash, &settings.api_key.as_str())?;
            }
          }
        }

        else {

          if self.vt == true {
            if settings.auto_search_vt.clone() == false {
              println!("{}: Virus Total search must be enabled in the config file", style("Error").red().bright());
            }

            else {
              println!("Querying [{}] on Virus Total", style(settings.file_hash.clone()).cyan());
              Self::search_virus_total(&settings.file_hash, &settings.api_key.as_str())?;
            }
          }

          if self.exports == true {
            let export_table = Self::get_exports(&exports);
            println!("{export_table}");
          }

          if self.ex_libs == true {
            let export_lib_table = Self::get_exported_lib(exports);
            println!("{export_lib_table}");
          }

          if self.imports == true {
            let import_table = Self::get_imports(&imports);
            println!("{import_table}");
          }

          if self.sections == true {
            Self::get_section_data(sections);
          }

          if self.dos_header == true {
            let dos_header_tb = Self::get_dos_header(dos_header);
            println!("{dos_header_tb}");
          }

          if self.coff_header == true {
            let coff_header_tb = Self::get_coff_header(coff_header);
            println!("{coff_header_tb}");
          }

          if self.optional_header == true {
            match Self::get_optional_header(optional_header) {
              Some(table) => {
                println!("{table}");
              }
              None => {
                println!("Could not read header");
              }
            }
          }

          if self.directories == true {
            match optional_header.clone() {
              Some(h) => {
                let table = Self::get_data_directories(h.data_directories);
                println!("{table}");
              }
              None => {
                println!("Could not read data directories");
              }
            }
          }
        }
      },

      Object::Mach(_) => {},
      Object::Archive(_) => {},
      Object::Unknown(magic) => { println!("unknown magic: {:#x}", magic) }
    }

    Ok(())
  }

  /**Function returns information stored in the dos header.
   * Params:
   *  header: DosHeader {The dos header}
   * Returns Table
   */
  pub fn get_dos_header(header: DosHeader) -> Table {
    let mut table = Table::new();
    table.set_header(vec![
      Cell::from("Header_Name").fg(Color::Green),
      Cell::from("Header_Offset").fg(Color::DarkCyan),
      Cell::from("Signature").fg(Color::DarkYellow),
    ]);

    table.add_row(vec![
      Cell::from("DOS_Header").fg(Color::Green),
      Cell::from(format!("0x{:x}", header.pe_pointer)).fg(Color::DarkCyan),
      Cell::from(format!("0x{:x}", header.signature)).fg(Color::DarkYellow),
    ]);

    table
  }

  /**Function returns information about the coff_header.
   * Params:
   *  header: CoffHeader {The CoffHeader}
   * Returns Table
   */
  pub fn get_coff_header(header: CoffHeader) -> Table {
    let mut table = Table::new();
    table.add_row(vec![
      Cell::from("Header_Name"),
      Cell::from("Characteristics").fg(Color::Blue),
      Cell::from("Machine Type").fg(Color::Green),
      Cell::from("Number_Of_Sections").fg(Color::DarkYellow),
      Cell::from("Number_Of_Symbols").fg(Color::DarkCyan),
      Cell::from("Symbol_Address_Table").fg(Color::Red),
      Cell::from("Optional_Header_Size").fg(Color::Cyan),
      Cell::from("Timestamp").fg(Color::Yellow),
    ]);

    table.add_row(vec![
      Cell::from("COFF_Header"),
      Cell::from(format!("0x{:x}", header.characteristics)).fg(Color::Blue),
      Cell::from(format!("0x{:x}", header.machine)).fg(Color::Green),
      Cell::from(header.number_of_sections).fg(Color::DarkYellow),
      Cell::from(header.number_of_symbol_table).fg(Color::DarkCyan),
      Cell::from(format!("0x{:x}", header.pointer_to_symbol_table)).fg(Color::Red),
      Cell::from(format!("0x{:x} ({} bytes)", header.size_of_optional_header, header.size_of_optional_header)).fg(Color::Cyan),
      Cell::from(header.time_date_stamp).fg(Color::Yellow),
    ]);

    table
  }

  /**Function creates vec of data directories with their names, sizes and virtual addresses.
   * Params:
   *  dir: DataDirectories {The data directories to check}
   * Returns Vec<DataDirectoryInfo>
   */
  pub fn get_data_directories(dir: DataDirectories) -> Table {
    let mut table = Table::new();
    table.set_header(vec![
      Cell::from("Name").fg(Color::DarkCyan),
      Cell::from("Size").fg(Color::Yellow),
      Cell::from("Virtual_Address").fg(Color::Yellow),
    ]);

    let mut label_string = String::new();
    let mut size_string = String::new();
    let mut rva_string = String::new();

    let mut directories: Vec<DataDirectoryInfo> = Default::default();
    let mut arch = DataDirectoryInfo::default();
    let mut realloc = DataDirectoryInfo::default();
    let mut bound_import = DataDirectoryInfo::default();
    let mut certificate = DataDirectoryInfo::default();
    let mut clr_runtime_header = DataDirectoryInfo::default();
    let mut debug = DataDirectoryInfo::default();
    let mut delay_import_desc = DataDirectoryInfo::default();
    let mut except = DataDirectoryInfo::default();
    let mut export = DataDirectoryInfo::default();
    let mut global_ptr = DataDirectoryInfo::default();
    let mut import_addr = DataDirectoryInfo::default();
    let mut import = DataDirectoryInfo::default();
    let mut load_config = DataDirectoryInfo::default();
    let mut resource = DataDirectoryInfo::default();
    let mut tls = DataDirectoryInfo::default();

    if let Some(d) = dir.get_architecture() {
      arch.name.push_str("architecture");
      arch.size = d.size;
      arch.virtual_address =d.virtual_address;
      directories.push(arch);
    }

    if let Some(d) = dir.get_base_relocation_table() {
      realloc.name.push_str("relocation");
      realloc.size = d.size;
      realloc.virtual_address =d.virtual_address;
      directories.push(realloc);
    }

    if let Some(d) = dir.get_bound_import_table() {
      bound_import.name.push_str("bound_import");
      bound_import.size = d.size;
      bound_import.virtual_address =d.virtual_address;
      directories.push(bound_import);
    }

    if let Some(d) = dir.get_certificate_table() {
      certificate.name.push_str("certificate");
      certificate.size = d.size;
      certificate.virtual_address =d.virtual_address;
      directories.push(certificate);
    }

    if let Some(d) = dir.get_clr_runtime_header() {
      clr_runtime_header.name.push_str("clr_runtime_header");
      clr_runtime_header.size = d.size;
      clr_runtime_header.virtual_address =d.virtual_address;
      directories.push(clr_runtime_header);
    }

    if let Some(d) = dir.get_debug_table() {
      debug.name.push_str("debug");
      debug.size = d.size;
      debug.virtual_address =d.virtual_address;
      directories.push(debug);
    }

    if let Some(d) = dir.get_delay_import_descriptor() {
      delay_import_desc.name.push_str("delay_import");
      delay_import_desc.size = d.size;
      delay_import_desc.virtual_address =d.virtual_address;
      directories.push(delay_import_desc);
    }

    if let Some(d) = dir.get_exception_table() {
      except.name.push_str("exception");
      except.size = d.size;
      except.virtual_address =d.virtual_address;
      directories.push(except);
    }

    if let Some(d) = dir.get_export_table() {
      export.name.push_str("export");
      export.size = d.size;
      export.virtual_address =d.virtual_address;
      directories.push(export);
    }

    if let Some(d) = dir.get_global_ptr() {
      global_ptr.name.push_str("global_ptr");
      global_ptr.size = d.size;
      global_ptr.virtual_address =d.virtual_address;
      directories.push(global_ptr);
    }

    if let Some(d) = dir.get_import_address_table() {
      import_addr.name.push_str("import_address");
      import_addr.size = d.size;
      import_addr.virtual_address =d.virtual_address;
      directories.push(import_addr);
    }

    if let Some(d) = dir.get_import_table() {
      import.name.push_str("import");
      import.size = d.size;
      import.virtual_address =d.virtual_address;
      directories.push(import);
    }

    if let Some(d) = dir.get_load_config_table() {
      load_config.name.push_str("load_config");
      load_config.size = d.size;
      load_config.virtual_address =d.virtual_address;
      directories.push(load_config);
    }

    if let Some(d) = dir.get_resource_table() {
      resource.name.push_str("resource");
      resource.size = d.size;
      resource.virtual_address =d.virtual_address;
      directories.push(resource);
    }

    if let Some(d) = dir.get_tls_table() {
      tls.name.push_str("tls");
      tls.size = d.size;
      tls.virtual_address =d.virtual_address;
      directories.push(tls);
    }

    for i in directories.clone() {
      label_string.push_str(format!("{}\n", i.name).as_str());
      size_string.push_str(format!("0x{:x} ({} bytes)\n", i.size, i.size).as_str());
      rva_string.push_str(format!("0x{:x}\n", i.virtual_address).as_str());
    }

    label_string.pop();
    size_string.pop();
    rva_string.pop();

    table.add_row(vec![
      Cell::from(label_string).fg(Color::DarkCyan),
      Cell::from(size_string).fg(Color::Yellow),
      Cell::from(rva_string).fg(Color::Yellow),
    ]);

    table
  }

  /**Function returns information about the optional_header.
   * Params:
   *  header: Option<OptionalHeader> {The optional header}
   * Returns Option<Table>
   */
  pub fn get_optional_header(header: Option<OptionalHeader>) -> Option<Table> {
    if let Some(h) = header {

      println!("{:#?}", h.data_directories.data_directories);
      let test = goblin::pe::header::Header::default().optional_header.unwrap().data_directories;

      let mut labels: Vec<&str> = Default::default();
      labels.push("magic");
      labels.push("major_linker_version");
      labels.push("minor_linker_version");
      labels.push("size_of_code");
      labels.push("size_of_initialized_data");
      labels.push("size_of_uninitialized_data");
      labels.push("address_of_entry_point");
      labels.push("base_of_code");
      labels.push("base_of_data");

      labels.push("check_sum");
      labels.push("dll_characteristics");
      labels.push("file_alignment");
      labels.push("image_base");
      labels.push("loader_flags");
      labels.push("major_image_version");
      labels.push("major_operating_system_version");
      labels.push("major_subsystem_version");
      labels.push("minor_image_version");
      labels.push("minor_operating_system_version");
      labels.push("minor_subsystem_version");
      labels.push("number_of_rva_and_sizes");
      labels.push("section_alignment");
      labels.push("size_of_headers");
      labels.push("size_of_heap_commit");
      labels.push("size_of_heap_reserve");
      labels.push("size_of_image");
      labels.push("size_of_stack_commit");
      labels.push("size_of_stack_reserve");
      labels.push("subsystem");
      labels.push("win32_version_value");

      let mut table = Table::new();
      let mut label_string = String::new();
      let mut value_string = String::new();

      for i in labels.clone() {
        label_string.push_str(format!("{}\n", i).as_str());
      }

      label_string.pop();

      let mut values: Vec<String> = Default::default();
      values.push(format!("{}\n", h.standard_fields.magic));
      values.push(format!("{}\n", h.standard_fields.major_linker_version));
      values.push(format!("{}\n", h.standard_fields.minor_linker_version));
      values.push(format!("{} bytes\n", h.standard_fields.size_of_code));
      values.push(format!("{} bytes\n", h.standard_fields.size_of_initialized_data));
      values.push(format!("{} bytes\n", h.standard_fields.size_of_uninitialized_data));
      values.push(format!("0x{:x}\n", h.standard_fields.address_of_entry_point));
      values.push(format!("0x{:x}\n", h.standard_fields.base_of_code));
      values.push(format!("0x{:x}\n", h.standard_fields.base_of_data));
      values.push(format!("0x{:x}\n", h.windows_fields.check_sum));
      values.push(format!("0x{:x}\n", h.windows_fields.dll_characteristics));
      values.push(format!("0x{:x}\n", h.windows_fields.file_alignment));
      values.push(format!("0x{:x}\n", h.windows_fields.image_base));
      values.push(format!("0x{:x}\n", h.windows_fields.loader_flags));
      values.push(format!("{}\n", h.windows_fields.major_image_version));
      values.push(format!("{}\n", h.windows_fields.major_operating_system_version));
      values.push(format!("{}\n", h.windows_fields.major_subsystem_version));
      values.push(format!("{}\n", h.windows_fields.minor_image_version));
      values.push(format!("{}\n", h.windows_fields.minor_operating_system_version));
      values.push(format!("{}\n", h.windows_fields.minor_subsystem_version));
      values.push(format!("{}\n", h.windows_fields.number_of_rva_and_sizes));
      values.push(format!("{}\n", h.windows_fields.section_alignment));
      values.push(format!("{} bytes\n", h.windows_fields.size_of_headers));
      values.push(format!("{} bytes\n", h.windows_fields.size_of_heap_commit));
      values.push(format!("{} bytes\n", h.windows_fields.size_of_heap_reserve));
      values.push(format!("{} bytes\n", h.windows_fields.size_of_image));
      values.push(format!("{} bytes\n", h.windows_fields.size_of_stack_commit));
      values.push(format!("{} bytes\n", h.windows_fields.size_of_stack_reserve));
      values.push(format!("{}\n", h.windows_fields.subsystem));
      values.push(format!("0x{:x}", h.windows_fields.win32_version_value));

      for i in values.clone() {
        value_string.push_str(i.as_str());
      }

      let row = Row::from(vec![
        Cell::from(label_string).fg(Color::DarkCyan),
        Cell::from(value_string).fg(Color::Yellow),
      ]);

      table.add_row(row);
      return Some(table);
    }

    None
  }

  /**Function displays information about each section header in the form of a table.
   * Params:
   *  sections: Vec<SectionTable> {The headers and information to be displayed}
   * Returns Table
   */
  pub fn get_section_data(sections: Vec<SectionTable>) -> Table {
    let mut table = Table::new();

    table.set_header(vec![
      Cell::from("Name"), 
      Cell::from("Characteristics").fg(Color::Green), 
      Cell::from("Virtual_Address").fg(Color::DarkCyan),
      Cell::from("Virtual_Size").fg(Color::DarkYellow), 
      Cell::from("Raw_Address").fg(Color::Cyan),
      Cell::from("Raw_Size").fg(Color::Red),
    ]);

    for i in sections.clone() {
      let mut section_name = "";
      
      match i.name() {
        Ok(s) => {
          section_name = s;
        },
        Err(e) => {}
      }

      let mut ptr_rw_data = String::new();
      match i.pointer_to_raw_data {
        0 => { ptr_rw_data.push_str("None"); }
        _ => { ptr_rw_data.push_str(format!("{:?}", i.pointer_to_raw_data as *const u32).as_str()); }
      }

      table.add_row(vec![
        Cell::from(section_name), 
        Cell::from(format!("0x{:x}", i.characteristics)).fg(Color::Green), 
        Cell::from(format!("0x{:x}", i.virtual_address)).fg(Color::DarkCyan),
        Cell::from(format!("{:x} ({} bytes)", i.virtual_size, i.virtual_size)).fg(Color::DarkYellow), 
        Cell::from(ptr_rw_data).fg(Color::Cyan),
        Cell::from(format!("{:x} ({} bytes)",i.size_of_raw_data, i.size_of_raw_data)).fg(Color::Red), 
      ]);
    }

    println!("{table}");
    table
  }

  /**Function makes a GET reuqest to the virus total api query a hash for the input file.
   * Params:
   *  hash_id: &str {The file hash}
   *  apikey: &str  {The Virus Total api key}
   * Returns nothing
   */
  pub fn search_virus_total(hash_id: &str, apikey: &str) -> std::io::Result<()> {
    let base_url = format!("https://www.virustotal.com/api/v3/files/{hash_id}");
  
    let builder = ClientBuilder::new()
                                  .build().unwrap().request(Method::GET, base_url)
                                  .header("x-apikey", apikey);
    
    let request = builder.send().unwrap();
    let text = request.text().unwrap();

    // Deserialize the json object in another thread.
    let (tx, rx) = std::sync::mpsc::channel::<VtJsonOutput>();
    std::thread::spawn(Box::new(move || {
      match serde_json::from_str::<VtJsonOutput>(&text) {
        Ok(s) => {

          // Send the results back to the main thread.
          match tx.send(s) {
            Ok(_) => {},
            Err(e) => {
              println!("{e}");
            }
          }
        },
        Err(e) => {
          println!("{e}");
        }
      }
    }));

    // receive the data.
    let mut output_data = VtJsonOutput::default();
    match rx.recv() {
      Ok(s) => {
        output_data = s;
      },
      Err(e) => {}
    }
    
    // Setup a table.
    let mut table = Table::new();
    table.set_header(vec![
      Cell::from("Av_Engine").fg(Color::Yellow),
      Cell::from("Category").fg(Color::Green),
      Cell::from("Result").fg(Color::Red),
      Cell::from("Version").fg(Color::DarkCyan),
      Cell::from("Method").fg(Color::DarkYellow),
      Cell::from("Engine_Update").fg(Color::Magenta),
    ]);

    // Unpack the AnalysisResult struct and construct the table.
    let av = Self::get_av_provider_data(output_data.data.attributes.last_analysis_results);
    for i in av {
      let c_av = Cell::from(i.engine_name).fg(Color::Yellow);
      let c_method = Cell::from(i.method).fg(Color::DarkYellow);
      let c_update = Cell::from(i.engine_update).fg(Color::Magenta);
      
      let category = i.category;
      let mut result = String::from("None");
      let mut version = String::from("None");
      
      let mut c_category = Cell::from(category.clone());
      let mut c_result = Cell::from(result.clone());
      let mut c_version = Cell::from(version.clone());

      if let Some(r) = i.result {
        result.clear();
        result.push_str(r.as_str());
      }

      if let Some(v) = i.engine_version {
        version.clear();
        version.push_str(v.as_str());
      }

      match category.as_str() {
        "type-unsupported" => { c_category = Cell::from(category).fg(Color::Blue); }
        "undetected" =>       { c_category = Cell::from(category).fg(Color::Green); }
        "malicious" =>        { c_category = Cell::from(category).fg(Color::Red); }
        _ => {}
      }

      match result.as_str() {
        "None" => {}
        _ => { c_result = Cell::from(result).fg(Color::Red); }
      }

      match version.as_str() {
        "None" => {}
        _ => { c_version = Cell::from(version).fg(Color::DarkCyan); }
      }

      let row = Row::from(vec![
        c_av,
        c_category,
        c_result,
        c_version,
        c_method,
        c_update,
      ]);

      table.add_row(row);
    }

    println!("{table}");
    Ok(())
  }

  /**Function returns the library name from re_export debug string.
   * Params:
   *  buffer: &str {The data to filter}
   * Returns String.
   */
  pub fn filter_export_lib(buffer: &str) -> String {
    let filter: Vec<&str> = buffer.split("lib: ").collect();
    let mut last_chunk = filter[1]
                                .replace("\"", "")
                                .replace(",", "")
                                .replace(" ", "")
                                .replace("}", "");

    last_chunk.pop();
    last_chunk
  }

  /**Function returns a list of library names that were found in the export table.
   * Params:
   *  exports: Vec<Export> {The exported functions}
   * Returns Table.
   */
  pub fn get_exported_lib(exports: Vec<Export>) -> Table {
    let mut table = Table::new();
    let mut lib_names: Vec<String> = Default::default();

    table.set_header(vec![
      Cell::from("Library Name").fg(Color::Green),
    ]);

    for i in exports {
      if let Some(re) = i.reexport {
        let re_debug = Self::filter_export_lib(format!("{:#?}", re).as_str());
        lib_names.push(re_debug);
      }
    }

    lib_names.sort();
    lib_names.dedup();

    let mut lib_name_string = String::new();
    for i in lib_names {
      lib_name_string.push_str(format!("{}\n", i).as_str());
    }

    lib_name_string.pop();
    let row = Row::from(vec![
      Cell::from(lib_name_string).fg(Color::DarkCyan),
    ]);

    table.add_row(row);
    table
  }

  /**Function returns a list exported function found in the binary in a table like format.
   * Params:
   *  exports: &Vec<Export>
   * Returns Table
   */
  pub fn get_exports(exports: &Vec<Export>) -> Table {
    let mut table = Table::new();
  
    println!("{}: {}", style("Exports").cyan(), style(exports.len()).yellow().bright());
    table.set_header(
      vec![
        Cell::from("Name").fg(Color::Green), 
        Cell::from("Offset").fg(Color::DarkCyan), 
        Cell::from("RVA").fg(Color::DarkYellow),
      ]
    );
  
    let mut export_names = String::new();
    let mut lib_name = String::new();
    let mut offsets = String::new();
    let mut rvas = String::new();
  
    for i in exports {
      if let Some(n) = i.name {
        export_names.push_str(format!("{n}\n").as_str());
      }
      
      if let Some(off) = i.offset {
        offsets.push_str(format!("0x{off}\n").as_str());
      }
  
      rvas.push_str(format!("0x{}\n", i.rva).as_str());
    }
  
    export_names.pop();
    lib_name.pop();
    offsets.pop();
    rvas.pop();
  
    let row = Row::from(vec![
      Cell::from(export_names).fg(Color::Green),
      Cell::from(offsets).fg(Color::DarkCyan),
      Cell::from(rvas).fg(Color::DarkYellow),
    ]);
  
    table.add_row(row);
    table
  }

  /**Function returns a list of imported function found in the binary in a table like format.
   * Params:
   *  imports: &Vec<Import>
   * Returns Table
   */
  pub fn get_imports(imports: &Vec<Import>) -> Table {
    let mut table = Table::new();

    table.set_header(
      vec![
        Cell::from("Name").fg(Color::Green),
        Cell::from("DLL").fg(Color::DarkCyan),
        Cell::from("Offset").fg(Color::DarkYellow),
        Cell::from("RVA").fg(Color::Red),
      ]
    );

    let mut names = String::new();
    let mut dll_names = String::new();
    let mut offsets = String::new();
    let mut rvas = String::new();

    for i in imports {
      names.push_str(format!("{}\n", i.name).as_str());
      dll_names.push_str(format!("{}\n", i.dll).as_str());
      offsets.push_str(format!("{}\n", i.offset).as_str());
      rvas.push_str(format!("{}\n", i.offset).as_str());
    }

    names.pop();
    dll_names.pop();
    offsets.pop();
    rvas.pop();

    table.add_row(
      Row::from(vec![
        Cell::from(names).fg(Color::Green),
        Cell::from(dll_names).fg(Color::DarkCyan),
        Cell::from(offsets).fg(Color::DarkYellow),
        Cell::from(rvas).fg(Color::Red),
      ])
    );

    table
  }

  /**Function loads the config file into memory and reads the value for each key.
   * Params:
   *  settings: &mut CmdSettings {The config settings}
   * Returns Result<()>
   */
  pub fn load_config_file(settings: &mut CmdSettings) -> std::io::Result<()> {
    let mut string_buf = String::new();
    let path = Path::new(config_options::CONFIG_FILE);
    let buffer = read(path)?;

    match String::from_utf8(buffer) {
      Ok(s) => {
        string_buf.push_str(s.as_str());
      },
      Err(_) => {}
    }

    let mut split_buffer: Vec<&str> = Default::default();
    let pattern = Self::is_split_valid(string_buf.clone());
    match pattern {
      SplitType::CarriageNewLine => {
        split_buffer = string_buf.split("\r\n").collect();
      }

      SplitType::NewLine =>         {
        split_buffer = string_buf.split("\n").collect();
      }

      SplitType::Unknown =>         {
        
      }
    }

    for i in split_buffer.clone() {
      let line = String::from(i);

      let split_keys: Vec<&str> = line.split('=').collect();
      if split_keys.len() > 1 {
        Self::parse_config_file(split_keys, settings);
      }  
    }

    // println!("{:#?}", settings);
    Ok(())
  }

  /**Function parses each option in the config file and verifies if the provided values are correct.
   * Params:
   *  pair:     Vec<&str>        {The key and the value}
   *  settings: &mut CmdSettings {The config settings}
   * Returns nothing
   */
  pub fn parse_config_file(pair: Vec<&str>, settings: &mut CmdSettings) -> () {
    match pair[0] {
      config_options::ENABLE_VT_SEARCH => {
        match pair[1] {
          "true" =>   { settings.auto_search_vt = true; }
          "false" =>  { settings.auto_search_vt = false; }
          _ =>        {}
        }
      }

      config_options::API_KEY =>                    {
        if pair[1].len() >= 64 {
          settings.api_key.push_str(pair[1]);
        }
      }

      _ => {
        println!("{}? unknown configuration option", style(pair[1]).yellow().bright());
      }
    }
  }

  pub fn is_split_valid(buffer: String) -> SplitType {
    let mut test_split: Vec<&str> = buffer.split("\r\n").collect();
    if test_split.len() > 1 {
      return SplitType::CarriageNewLine;
    }

    test_split = buffer.split("\n").collect();
    if test_split.len() > 1 {
      return SplitType::NewLine;
    }

    return SplitType::Unknown;
  }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SplitType {
  NewLine,
  CarriageNewLine,
  Unknown,
}