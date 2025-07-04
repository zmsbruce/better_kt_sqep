#[cfg(feature = "extension-module")]
use pyo3::{exceptions::PyException, prelude::*};

mod app;
mod error;
mod file;
mod graph;

#[cfg(feature = "extension-module")]
#[pyclass(name = "KnowledgeGraph")]
pub struct PyKnowledgeGraph {
    graph: graph::KnowledgeGraph,
}

#[cfg(feature = "extension-module")]
#[pymethods]
impl PyKnowledgeGraph {
    #[new]
    fn new() -> Self {
        Self {
            graph: graph::KnowledgeGraph::default(),
        }
    }

    fn to_xml(&self) -> PyResult<String> {
        match self.graph.current.to_xml() {
            Ok(xml) => Ok(xml),
            Err(e) => Err(PyErr::new::<PyException, _>(format!("Internal error: {e}"))),
        }
    }

    fn add_entity(
        &mut self,
        content: String,
        distinct_type: String,
        addon_types: String,
        x: f64,
        y: f64,
    ) -> PyResult<u64> {
        // 将 distinct_type 转为 enum
        let distinct_type = match distinct_type.to_lowercase().as_str() {
            "ka" => graph::DistinctEntityType::KnowledgeArena,
            "ku" => graph::DistinctEntityType::KnowledgeUnit,
            "kp" => graph::DistinctEntityType::KnowledgePoint,
            "kd" => graph::DistinctEntityType::KnowledgeDetail,
            _ => {
                return Err(PyErr::new::<PyException, _>(format!(
                    "Invalid distinct type {distinct_type}"
                )));
            }
        };

        // 将 addon_types 转为 HashSet
        let addon_types = addon_types
            .to_lowercase()
            .chars()
            .map(|c| match c {
                'k' => Ok(graph::AddonEntityType::Knowledge),
                't' => Ok(graph::AddonEntityType::Thinking),
                'e' => Ok(graph::AddonEntityType::Example),
                'q' => Ok(graph::AddonEntityType::Question),
                'p' => Ok(graph::AddonEntityType::Practice),
                'z' => Ok(graph::AddonEntityType::Political),
                _ => Err(PyErr::new::<PyException, _>(format!(
                    "Invalid addon type {c}"
                ))),
            })
            .collect::<Result<Vec<_>, _>>()?;

        let id = self
            .graph
            .add_entity(content, distinct_type, &addon_types, (x, y));

        Ok(id)
    }

    fn add_edge(&mut self, from: u64, to: u64, relation: String) -> PyResult<()> {
        let relation = match relation.to_lowercase().as_str() {
            "contain" => graph::Relation::Contain,
            "order" => graph::Relation::Order,
            _ => {
                return Err(PyErr::new::<PyException, _>(format!(
                    "Invalid relation {relation}"
                )));
            }
        };

        self.graph
            .add_edge(from, to, relation)
            .map_err(|e| PyErr::new::<PyException, _>(format!("Internal error: {e}")))?;

        Ok(())
    }

    fn remove_entity(&mut self, id: u64) -> PyResult<()> {
        self.graph
            .remove_entity(id)
            .map_err(|e| PyErr::new::<PyException, _>(format!("Internal error: {e}")))?;

        Ok(())
    }

    fn remove_edge(&mut self, from: u64, to: u64) -> PyResult<()> {
        self.graph
            .remove_edge(from, to)
            .map_err(|e| PyErr::new::<PyException, _>(format!("Internal error: {e}")))?;

        Ok(())
    }
}

#[cfg(feature = "extension-module")]
#[pymodule]
pub fn py_better_kt_sqep(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyKnowledgeGraph>()?;
    Ok(())
}
