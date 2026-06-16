use super::commands::QaCommands;

pub fn run_qa(command: QaCommands) -> Result<(), String> {
    match command {
        QaCommands::Accuracy {
            case_dir,
            corpus_manifest,
            output_dir,
        } => {
            let output_dir = output_dir.unwrap_or_else(|| case_dir.join("reports/qa"));
            let report = crate::qa::accuracy_report(&case_dir, &corpus_manifest, &output_dir)?;
            println!("accuracy QA passed: {}", report.report_path.display());
            Ok(())
        }
        QaCommands::Reproducibility {
            left_case_dir,
            right_case_dir,
            output_dir,
        } => {
            let output_dir = output_dir.unwrap_or_else(|| left_case_dir.join("reports/qa"));
            let report =
                crate::qa::reproducibility_report(&left_case_dir, &right_case_dir, &output_dir)?;
            println!(
                "reproducibility QA passed: {}",
                report.report_path.display()
            );
            Ok(())
        }
        QaCommands::ReportDefense {
            case_dir,
            output_dir,
        } => {
            let output_dir = output_dir.unwrap_or_else(|| case_dir.join("reports/qa"));
            let report = crate::qa::report_defense_check(&case_dir, &output_dir)?;
            println!("report-defense QA passed: {}", report.report_path.display());
            Ok(())
        }
        QaCommands::Performance { output_dir, rows } => {
            let report = crate::qa::performance_report(&output_dir, rows)?;
            println!("performance QA passed: {}", report.report_path.display());
            Ok(())
        }
        QaCommands::Release {
            case_dir,
            corpus_manifest,
            comparison_case,
            review_manifest,
            output_dir,
            performance_output_dir,
            performance_rows,
        } => {
            let output_dir = output_dir.unwrap_or_else(|| case_dir.join("reports/qa"));
            let options = crate::qa::ReleaseReadinessOptions {
                corpus_manifest,
                comparison_case_dir: comparison_case,
                review_manifest,
                performance_output_dir,
                performance_rows,
            };
            let report = crate::qa::release_readiness_report(&case_dir, &output_dir, &options)?;
            println!(
                "release readiness QA passed: {}",
                report.report_path.display()
            );
            Ok(())
        }
    }
}
