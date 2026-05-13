use crate::config::PackageConfig;
use crate::error::{GraftError, Result};
use crate::platform::Platform;
use std::collections::{HashMap, HashSet, VecDeque};

/// Resolves the processing order for packages via topological sort.
/// Returns an ordered list where dependencies come before dependents.
pub fn resolve_order(
    packages: &HashMap<String, &PackageConfig>,
    requested: &[String],
    _current_platform: &Platform,
) -> Result<Vec<String>> {
    // Determine which packages to include
    let roots: Vec<String> = if requested.is_empty() {
        packages.keys().cloned().collect()
    } else {
        requested.to_vec()
    };

    // Collect all needed packages (roots + transitive deps)
    let mut needed: HashSet<String> = HashSet::new();
    let mut stack: Vec<String> = roots;

    while let Some(name) = stack.pop() {
        if !needed.insert(name.clone()) {
            continue;
        }
        let pkg = packages
            .get(&name)
            .ok_or_else(|| GraftError::MissingDependency {
                package: name.clone(),
                dependency: name.clone(),
            })?;
        if let Some(deps) = &pkg.depends_on {
            for dep in deps {
                if !packages.contains_key(dep) {
                    return Err(GraftError::MissingDependency {
                        package: name.clone(),
                        dependency: dep.clone(),
                    });
                }
                stack.push(dep.clone());
            }
        }
    }

    // Build adjacency and in-degree for Kahn's algorithm (only over needed set)
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();

    for name in &needed {
        in_degree.entry(name.as_str()).or_insert(0);
        dependents.entry(name.as_str()).or_default();
    }

    for name in &needed {
        if let Some(deps) = &packages[name.as_str()].depends_on {
            for dep in deps {
                if needed.contains(dep) {
                    dependents
                        .entry(dep.as_str())
                        .or_default()
                        .push(name.as_str());
                    *in_degree.entry(name.as_str()).or_insert(0) += 1;
                }
            }
        }
    }

    // Kahn's algorithm
    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(name, _)| *name)
        .collect();

    let mut order: Vec<String> = Vec::new();

    while let Some(node) = queue.pop_front() {
        order.push(node.to_string());
        if let Some(deps) = dependents.get(node) {
            for &dependent in deps {
                if let Some(deg) = in_degree.get_mut(dependent) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(dependent);
                    }
                }
            }
        }
    }

    // If not all nodes processed, there's a cycle
    if order.len() != needed.len() {
        let cycle = find_cycle(&needed, packages);
        return Err(GraftError::CycleDetected(cycle));
    }

    Ok(order)
}

/// Finds a cycle in the dependency graph for error reporting.
fn find_cycle(needed: &HashSet<String>, packages: &HashMap<String, &PackageConfig>) -> Vec<String> {
    let mut visited: HashSet<&str> = HashSet::new();
    let mut on_stack: HashSet<&str> = HashSet::new();
    let mut path: Vec<&str> = Vec::new();

    for name in needed {
        if !visited.contains(name.as_str())
            && let Some(cycle) = dfs_cycle(
                name,
                packages,
                needed,
                &mut visited,
                &mut on_stack,
                &mut path,
            ) {
                return cycle;
            }
    }
    // Fallback (shouldn't reach here if called when cycle exists)
    needed.iter().cloned().collect()
}

fn dfs_cycle<'a>(
    node: &'a str,
    packages: &'a HashMap<String, &PackageConfig>,
    needed: &'a HashSet<String>,
    visited: &mut HashSet<&'a str>,
    on_stack: &mut HashSet<&'a str>,
    path: &mut Vec<&'a str>,
) -> Option<Vec<String>> {
    visited.insert(node);
    on_stack.insert(node);
    path.push(node);

    if let Some(pkg) = packages.get(node)
        && let Some(deps) = &pkg.depends_on {
            for dep in deps {
                if !needed.contains(dep) {
                    continue;
                }
                if on_stack.contains(dep.as_str()) {
                    // Found cycle — extract it
                    let start = path.iter().position(|&n| n == dep.as_str()).unwrap();
                    let mut cycle: Vec<String> =
                        path[start..].iter().map(|s| s.to_string()).collect();
                    cycle.push(dep.clone());
                    return Some(cycle);
                }
                if !visited.contains(dep.as_str())
                    && let Some(cycle) = dfs_cycle(dep, packages, needed, visited, on_stack, path) {
                        return Some(cycle);
                    }
            }
        }

    path.pop();
    on_stack.remove(node);
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal stub for tests since config.rs may not be complete yet
    fn make_pkg(depends_on: Option<Vec<&str>>) -> PackageConfig {
        PackageConfig {
            os: None,
            depends_on: depends_on.map(|d| d.into_iter().map(String::from).collect()),
            install: None,
            install_command: None,
            files: None,
            link_mode: None,
            tags: None,
        }
    }

    #[test]
    fn test_simple_linear_dependency() {
        let a = make_pkg(Some(vec!["b"]));
        let b = make_pkg(None);
        let packages: HashMap<String, &PackageConfig> =
            HashMap::from([("a".into(), &a), ("b".into(), &b)]);

        let result = resolve_order(&packages, &["a".into()], &Platform::new("macos")).unwrap();
        assert_eq!(result, vec!["b", "a"]);
    }

    #[test]
    fn test_cycle_detection() {
        let a = make_pkg(Some(vec!["b"]));
        let b = make_pkg(Some(vec!["a"]));
        let packages: HashMap<String, &PackageConfig> =
            HashMap::from([("a".into(), &a), ("b".into(), &b)]);

        let result = resolve_order(&packages, &["a".into()], &Platform::new("macos"));
        assert!(matches!(result, Err(GraftError::CycleDetected(_))));
    }

    #[test]
    fn test_missing_dependency() {
        let a = make_pkg(Some(vec!["missing"]));
        let packages: HashMap<String, &PackageConfig> = HashMap::from([("a".into(), &a)]);

        let result = resolve_order(&packages, &["a".into()], &Platform::new("macos"));
        assert!(matches!(result, Err(GraftError::MissingDependency { .. })));
        if let Err(GraftError::MissingDependency {
            package,
            dependency,
        }) = result
        {
            assert_eq!(package, "a");
            assert_eq!(dependency, "missing");
        }
    }

    #[test]
    fn test_no_dependencies() {
        let a = make_pkg(None);
        let b = make_pkg(None);
        let packages: HashMap<String, &PackageConfig> =
            HashMap::from([("a".into(), &a), ("b".into(), &b)]);

        let result = resolve_order(&packages, &[], &Platform::new("macos")).unwrap();
        // Both should be present, order doesn't matter
        assert_eq!(result.len(), 2);
        assert!(result.contains(&"a".to_string()));
        assert!(result.contains(&"b".to_string()));
    }

    #[test]
    fn test_transitive_dependencies() {
        let a = make_pkg(Some(vec!["b"]));
        let b = make_pkg(Some(vec!["c"]));
        let c = make_pkg(None);
        let packages: HashMap<String, &PackageConfig> =
            HashMap::from([("a".into(), &a), ("b".into(), &b), ("c".into(), &c)]);

        let result = resolve_order(&packages, &["a".into()], &Platform::new("macos")).unwrap();
        assert_eq!(result, vec!["c", "b", "a"]);
    }
}
