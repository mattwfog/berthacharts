//! Three-dimensional Gaussian models for dense statistical visualization.
//!
//! The model intentionally exposes small value types instead of pulling in a
//! linear algebra dependency. It is meant to support chart-side analysis:
//! fitting a 3D normal distribution, scoring observations, building density
//! volumes, and producing confidence ellipsoid meshes.

use thiserror::Error;

use berthacharts_core::Dataset;

const TWO_PI: f64 = core::f64::consts::PI * 2.0;
const MIN_CONFIDENCE: f64 = 1.0e-12;
const MAX_CONFIDENCE: f64 = 1.0 - 1.0e-12;

/// A point or vector in 3D statistical space.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Vec3 {
    /// X coordinate.
    pub x: f64,
    /// Y coordinate.
    pub y: f64,
    /// Z coordinate.
    pub z: f64,
}

impl Vec3 {
    /// Build a 3D vector.
    #[must_use]
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Coordinate by index: `0 => x`, `1 => y`, `2 => z`.
    #[must_use]
    pub const fn get(self, index: usize) -> Option<f64> {
        match index {
            0 => Some(self.x),
            1 => Some(self.y),
            2 => Some(self.z),
            _ => None,
        }
    }

    /// Return a copy with one coordinate replaced.
    #[must_use]
    pub const fn with(self, index: usize, value: f64) -> Option<Self> {
        match index {
            0 => Some(Self::new(value, self.y, self.z)),
            1 => Some(Self::new(self.x, value, self.z)),
            2 => Some(Self::new(self.x, self.y, value)),
            _ => None,
        }
    }

    /// Return true when every coordinate is finite.
    #[must_use]
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }

    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }

    fn scale(self, rhs: f64) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }

    fn dot(self, rhs: Self) -> f64 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }

    fn normalize(self) -> Self {
        let norm = self.norm();
        if norm == 0.0 {
            self
        } else {
            self.scale(1.0 / norm)
        }
    }
}

/// A row-major 3x3 matrix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mat3 {
    /// Matrix rows.
    pub rows: [[f64; 3]; 3],
}

impl Mat3 {
    /// Build a matrix from row-major values.
    #[must_use]
    pub const fn new(rows: [[f64; 3]; 3]) -> Self {
        Self { rows }
    }

    /// Build a diagonal matrix.
    #[must_use]
    pub const fn diagonal(x: f64, y: f64, z: f64) -> Self {
        Self::new([[x, 0.0, 0.0], [0.0, y, 0.0], [0.0, 0.0, z]])
    }

    /// Return true when every entry is finite.
    #[must_use]
    pub fn is_finite(self) -> bool {
        self.rows.iter().flatten().all(|value| value.is_finite())
    }

    /// Matrix determinant.
    #[must_use]
    pub fn determinant(self) -> f64 {
        let m = self.rows;
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    }

    /// Matrix inverse, if non-singular.
    #[must_use]
    pub fn inverse(self) -> Option<Self> {
        let m = self.rows;
        let det = self.determinant();
        if !det.is_finite() || det.abs() <= f64::EPSILON {
            return None;
        }
        let inv_det = 1.0 / det;
        let rows = [
            [
                (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * inv_det,
                (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * inv_det,
                (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * inv_det,
            ],
            [
                (m[1][2] * m[2][0] - m[1][0] * m[2][2]) * inv_det,
                (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * inv_det,
                (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * inv_det,
            ],
            [
                (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * inv_det,
                (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * inv_det,
                (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * inv_det,
            ],
        ];
        Some(Self::new(rows))
    }

    /// Lower Cholesky factor `L` where `self = L * L^T`, if positive definite.
    #[must_use]
    pub fn cholesky_lower(self) -> Option<Self> {
        let a = self.rows;
        let mut l = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..=i {
                let mut sum = a[i][j];
                for (&lik, &ljk) in l[i][..j].iter().zip(&l[j][..j]) {
                    sum -= lik * ljk;
                }
                if i == j {
                    if sum <= 0.0 || !sum.is_finite() {
                        return None;
                    }
                    l[i][j] = sum.sqrt();
                } else {
                    l[i][j] = sum / l[j][j];
                }
            }
        }
        Some(Self::new(l))
    }

    /// Return true when the matrix is symmetric within `tolerance`.
    #[must_use]
    pub fn is_symmetric(self, tolerance: f64) -> bool {
        (self.rows[0][1] - self.rows[1][0]).abs() <= tolerance
            && (self.rows[0][2] - self.rows[2][0]).abs() <= tolerance
            && (self.rows[1][2] - self.rows[2][1]).abs() <= tolerance
    }

    /// Eigen decomposition for symmetric 3x3 matrices.
    pub fn symmetric_eigen(self) -> Result<SymmetricEigen3, Gaussian3Error> {
        if !self.is_finite() {
            return Err(Gaussian3Error::NonFiniteCovariance);
        }
        if !self.is_symmetric(1.0e-10) {
            return Err(Gaussian3Error::NonsymmetricCovariance);
        }

        let mut a = self.rows;
        let mut v = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

        for _ in 0..48 {
            let (p, q, off_diag) = largest_off_diagonal(a);
            if off_diag <= 1.0e-12 {
                break;
            }

            let app = a[p][p];
            let aqq = a[q][q];
            let apq = a[p][q];
            let tau = (aqq - app) / (2.0 * apq);
            let t = if tau >= 0.0 {
                1.0 / (tau + (1.0 + tau * tau).sqrt())
            } else {
                -1.0 / (-tau + (1.0 + tau * tau).sqrt())
            };
            let c = 1.0 / (1.0 + t * t).sqrt();
            let s = t * c;

            for k in [0, 1, 2] {
                if k != p && k != q {
                    let akp = a[k][p];
                    let akq = a[k][q];
                    a[k][p] = c * akp - s * akq;
                    a[p][k] = a[k][p];
                    a[k][q] = s * akp + c * akq;
                    a[q][k] = a[k][q];
                }
            }

            a[p][p] = c * c * app - 2.0 * s * c * apq + s * s * aqq;
            a[q][q] = s * s * app + 2.0 * s * c * apq + c * c * aqq;
            a[p][q] = 0.0;
            a[q][p] = 0.0;

            for row in &mut v {
                let vip = row[p];
                let viq = row[q];
                row[p] = c * vip - s * viq;
                row[q] = s * vip + c * viq;
            }
        }

        let mut pairs = [
            (a[0][0], Vec3::new(v[0][0], v[1][0], v[2][0]).normalize()),
            (a[1][1], Vec3::new(v[0][1], v[1][1], v[2][1]).normalize()),
            (a[2][2], Vec3::new(v[0][2], v[1][2], v[2][2]).normalize()),
        ];
        pairs.sort_by(|left, right| right.0.total_cmp(&left.0));

        Ok(SymmetricEigen3 {
            values: [pairs[0].0, pairs[1].0, pairs[2].0],
            vectors: [pairs[0].1, pairs[1].1, pairs[2].1],
        })
    }

    fn add_diagonal(self, amount: f64) -> Self {
        let mut rows = self.rows;
        rows[0][0] += amount;
        rows[1][1] += amount;
        rows[2][2] += amount;
        Self::new(rows)
    }

    fn mul_vec(self, rhs: Vec3) -> Vec3 {
        let m = self.rows;
        Vec3::new(
            m[0][0] * rhs.x + m[0][1] * rhs.y + m[0][2] * rhs.z,
            m[1][0] * rhs.x + m[1][1] * rhs.y + m[1][2] * rhs.z,
            m[2][0] * rhs.x + m[2][1] * rhs.y + m[2][2] * rhs.z,
        )
    }
}

/// Eigenvalues and unit eigenvectors for a symmetric 3x3 matrix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SymmetricEigen3 {
    /// Eigenvalues sorted descending.
    pub values: [f64; 3],
    /// Unit eigenvectors corresponding to `values`.
    pub vectors: [Vec3; 3],
}

/// Covariance normalization used while fitting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CovarianceEstimator {
    /// Maximum-likelihood estimate, divides by `n`.
    MaximumLikelihood,
    /// Unbiased sample covariance, divides by `n - 1`.
    Unbiased,
}

/// Options for fitting a 3D Gaussian model from samples.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gaussian3FitOptions {
    /// Covariance normalization.
    pub estimator: CovarianceEstimator,
    /// Non-negative amount added to the covariance diagonal.
    pub regularization: f64,
}

impl Default for Gaussian3FitOptions {
    fn default() -> Self {
        Self {
            estimator: CovarianceEstimator::Unbiased,
            regularization: 1.0e-9,
        }
    }
}

/// A fitted full-covariance 3D Gaussian distribution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gaussian3 {
    /// Fitted mean vector.
    pub mean: Vec3,
    /// Full covariance matrix.
    pub covariance: Mat3,
    /// Cached covariance inverse.
    pub covariance_inverse: Mat3,
    /// Cached covariance determinant.
    pub determinant: f64,
}

impl Gaussian3 {
    /// Fit a full-covariance Gaussian from 3D samples.
    pub fn fit(samples: &[Vec3], options: Gaussian3FitOptions) -> Result<Self, Gaussian3Error> {
        if samples.len() < 2 {
            return Err(Gaussian3Error::InsufficientSamples {
                actual: samples.len(),
            });
        }
        if options.regularization < 0.0 || !options.regularization.is_finite() {
            return Err(Gaussian3Error::InvalidRegularization(
                options.regularization,
            ));
        }

        let mut mean = Vec3::default();
        for &sample in samples {
            if !sample.is_finite() {
                return Err(Gaussian3Error::NonFiniteSample(sample));
            }
            mean = mean.add(sample);
        }
        mean = mean.scale(1.0 / samples.len() as f64);

        let mut cov = [[0.0; 3]; 3];
        for &sample in samples {
            let d = sample.sub(mean);
            let values = [d.x, d.y, d.z];
            for row in 0..3 {
                for col in 0..3 {
                    cov[row][col] += values[row] * values[col];
                }
            }
        }

        let denominator = match options.estimator {
            CovarianceEstimator::MaximumLikelihood => samples.len() as f64,
            CovarianceEstimator::Unbiased => (samples.len() - 1) as f64,
        };
        for row in &mut cov {
            for value in row {
                *value /= denominator;
            }
        }

        let covariance = Mat3::new(cov).add_diagonal(options.regularization);
        Self::from_mean_covariance(mean, covariance)
    }

    /// Fit a full-covariance Gaussian from three numeric dataset columns.
    pub fn fit_dataset(
        dataset: &Dataset,
        x: &str,
        y: &str,
        z: &str,
        options: Gaussian3FitOptions,
    ) -> Result<Self, Gaussian3Error> {
        let samples = samples_from_dataset(dataset, x, y, z)?;
        Self::fit(&samples, options)
    }

    /// Build a Gaussian from explicit mean and covariance.
    pub fn from_mean_covariance(mean: Vec3, covariance: Mat3) -> Result<Self, Gaussian3Error> {
        if !mean.is_finite() {
            return Err(Gaussian3Error::NonFiniteMean(mean));
        }
        if !covariance.is_finite() {
            return Err(Gaussian3Error::NonFiniteCovariance);
        }
        if !covariance.is_symmetric(1.0e-10) {
            return Err(Gaussian3Error::NonsymmetricCovariance);
        }
        let determinant = covariance.determinant();
        if determinant <= 0.0 || !determinant.is_finite() {
            return Err(Gaussian3Error::SingularCovariance);
        }
        let covariance_inverse = covariance
            .inverse()
            .ok_or(Gaussian3Error::SingularCovariance)?;
        if covariance.cholesky_lower().is_none() {
            return Err(Gaussian3Error::SingularCovariance);
        }
        Ok(Self {
            mean,
            covariance,
            covariance_inverse,
            determinant,
        })
    }

    /// Squared Mahalanobis distance from the model center.
    #[must_use]
    pub fn mahalanobis_squared(&self, point: Vec3) -> f64 {
        let delta = point.sub(self.mean);
        delta.dot(self.covariance_inverse.mul_vec(delta))
    }

    /// Natural log probability density at a point.
    #[must_use]
    pub fn log_pdf(&self, point: Vec3) -> f64 {
        -0.5 * (3.0 * TWO_PI.ln() + self.determinant.ln() + self.mahalanobis_squared(point))
    }

    /// Probability density at a point.
    #[must_use]
    pub fn pdf(&self, point: Vec3) -> f64 {
        self.log_pdf(point).exp()
    }

    /// Return true when `point` lies inside the requested confidence ellipsoid.
    pub fn contains_confidence(
        &self,
        point: Vec3,
        confidence: f64,
    ) -> Result<bool, Gaussian3Error> {
        let radius = confidence_radius_3d(confidence)?;
        Ok(self.mahalanobis_squared(point) <= radius * radius)
    }

    /// Cumulative 3D Gaussian confidence mass at `point`.
    #[must_use]
    pub fn confidence_of(&self, point: Vec3) -> f64 {
        chi_square3_cdf(self.mahalanobis_squared(point))
    }

    /// Upper-tail probability beyond `point`'s Mahalanobis distance.
    #[must_use]
    pub fn tail_probability(&self, point: Vec3) -> f64 {
        (1.0 - self.confidence_of(point)).clamp(0.0, 1.0)
    }

    /// Negative log density, useful as an anomaly score.
    #[must_use]
    pub fn surprisal(&self, point: Vec3) -> f64 {
        -self.log_pdf(point)
    }

    /// Differential entropy in nats.
    #[must_use]
    pub fn differential_entropy(&self) -> f64 {
        0.5 * ((TWO_PI * core::f64::consts::E).powi(3) * self.determinant).ln()
    }

    /// Principal covariance axes sorted from largest to smallest variance.
    pub fn principal_axes(&self) -> Result<SymmetricEigen3, Gaussian3Error> {
        self.covariance.symmetric_eigen()
    }

    /// Fraction of total variance explained by each principal axis.
    pub fn variance_explained(&self) -> Result<[f64; 3], Gaussian3Error> {
        let axes = self.principal_axes()?;
        let total = axes.values.iter().sum::<f64>();
        if total <= 0.0 {
            return Err(Gaussian3Error::SingularCovariance);
        }
        Ok(axes.values.map(|value| value / total))
    }

    /// Covariance condition number, largest variance over smallest variance.
    pub fn condition_number(&self) -> Result<f64, Gaussian3Error> {
        let axes = self.principal_axes()?;
        if axes.values[2] <= 0.0 {
            return Err(Gaussian3Error::SingularCovariance);
        }
        Ok(axes.values[0] / axes.values[2])
    }

    /// Kullback-Leibler divergence `D_KL(self || other)` in nats.
    #[must_use]
    pub fn kl_divergence_to(&self, other: &Self) -> f64 {
        let delta = other.mean.sub(self.mean);
        0.5 * (trace_product(other.covariance_inverse, self.covariance)
            + delta.dot(other.covariance_inverse.mul_vec(delta))
            - 3.0
            + (other.determinant / self.determinant).ln())
    }

    /// Symmetric Bhattacharyya distance between two Gaussian models.
    pub fn bhattacharyya_distance(&self, other: &Self) -> Result<f64, Gaussian3Error> {
        let avg_covariance = average_covariance(self.covariance, other.covariance);
        let avg_inverse = avg_covariance
            .inverse()
            .ok_or(Gaussian3Error::SingularCovariance)?;
        let avg_determinant = avg_covariance.determinant();
        if avg_determinant <= 0.0 || !avg_determinant.is_finite() {
            return Err(Gaussian3Error::SingularCovariance);
        }
        let delta = other.mean.sub(self.mean);
        Ok(0.125 * delta.dot(avg_inverse.mul_vec(delta))
            + 0.5 * (avg_determinant / (self.determinant * other.determinant).sqrt()).ln())
    }

    /// Compact statistical summary for diagnostics and ranking.
    pub fn summary(&self, confidence: f64) -> Result<Gaussian3Summary, Gaussian3Error> {
        let ellipsoid = self.confidence_ellipsoid(confidence)?;
        Ok(Gaussian3Summary {
            mean: self.mean,
            determinant: self.determinant,
            entropy: self.differential_entropy(),
            condition_number: self.condition_number()?,
            variance_explained: self.variance_explained()?,
            ellipsoid_volume: ellipsoid.volume,
        })
    }

    /// Seven sigma points: center plus positive/negative principal covariance axes.
    pub fn sigma_points(&self, confidence: f64) -> Result<Gaussian3SigmaPointSet, Gaussian3Error> {
        let ellipsoid = self.confidence_ellipsoid(confidence)?;
        let mut points = Vec::with_capacity(7);
        points.push(Gaussian3SigmaPoint {
            point: self.mean,
            axis: None,
            sign: 0,
        });
        for axis in 0..3 {
            let offset = ellipsoid.axes.vectors[axis].scale(ellipsoid.radii[axis]);
            points.push(Gaussian3SigmaPoint {
                point: self.mean.add(offset),
                axis: Some(axis),
                sign: 1,
            });
            points.push(Gaussian3SigmaPoint {
                point: self.mean.sub(offset),
                axis: Some(axis),
                sign: -1,
            });
        }
        Ok(Gaussian3SigmaPointSet {
            confidence,
            radius: ellipsoid.mahalanobis_radius,
            points,
        })
    }

    /// Exact Shapley attributions using the model mean as the missing-feature baseline.
    #[must_use]
    pub fn shapley_values(&self, point: Vec3, score: Gaussian3ShapleyScore) -> Gaussian3Shapley {
        self.shapley_values_with_background(point, self.mean, score)
    }

    /// Exact Shapley attributions for the cooperative feature game.
    ///
    /// Coalition payoffs are computed by taking coordinates from `point` for
    /// features in the coalition and from `background` for missing features.
    #[must_use]
    pub fn shapley_values_with_background(
        &self,
        point: Vec3,
        background: Vec3,
        score: Gaussian3ShapleyScore,
    ) -> Gaussian3Shapley {
        let game = Gaussian3FeatureGame::new(*self, point, background, score);
        game.shapley_values()
    }

    /// Exact Shapley interaction matrix using the model mean as baseline.
    #[must_use]
    pub fn shapley_interactions(
        &self,
        point: Vec3,
        score: Gaussian3ShapleyScore,
    ) -> Gaussian3ShapleyInteraction {
        self.shapley_interactions_with_background(point, self.mean, score)
    }

    /// Exact Shapley interaction matrix for the cooperative feature game.
    #[must_use]
    pub fn shapley_interactions_with_background(
        &self,
        point: Vec3,
        background: Vec3,
        score: Gaussian3ShapleyScore,
    ) -> Gaussian3ShapleyInteraction {
        let game = Gaussian3FeatureGame::new(*self, point, background, score);
        game.shapley_interactions()
    }

    /// Descriptor for the requested confidence ellipsoid.
    pub fn confidence_ellipsoid(
        &self,
        confidence: f64,
    ) -> Result<Gaussian3Ellipsoid, Gaussian3Error> {
        let mahalanobis_radius = confidence_radius_3d(confidence)?;
        let axes = self.principal_axes()?;
        let radii = axes
            .values
            .map(|value| value.max(0.0).sqrt() * mahalanobis_radius);
        let volume = 4.0 * core::f64::consts::PI * radii[0] * radii[1] * radii[2] / 3.0;
        Ok(Gaussian3Ellipsoid {
            center: self.mean,
            confidence,
            mahalanobis_radius,
            axes,
            radii,
            volume,
        })
    }

    /// Generate a regular density grid over the provided 3D bounds.
    pub fn density_grid(
        &self,
        bounds: Bounds3,
        steps: [usize; 3],
    ) -> Result<Vec<Gaussian3DensityVoxel>, Gaussian3Error> {
        if !bounds.is_valid() {
            return Err(Gaussian3Error::InvalidBounds);
        }
        if steps.contains(&0) {
            return Err(Gaussian3Error::InvalidGrid);
        }

        let mut voxels = Vec::with_capacity(steps[0] * steps[1] * steps[2]);
        for ix in 0..steps[0] {
            let x = grid_value(bounds.min.x, bounds.max.x, ix, steps[0]);
            for iy in 0..steps[1] {
                let y = grid_value(bounds.min.y, bounds.max.y, iy, steps[1]);
                for iz in 0..steps[2] {
                    let z = grid_value(bounds.min.z, bounds.max.z, iz, steps[2]);
                    let position = Vec3::new(x, y, z);
                    voxels.push(Gaussian3DensityVoxel {
                        position,
                        density: self.pdf(position),
                    });
                }
            }
        }
        Ok(voxels)
    }

    /// Generate a confidence ellipsoid mesh.
    pub fn ellipsoid_mesh(
        &self,
        confidence: f64,
        latitude_segments: usize,
        longitude_segments: usize,
    ) -> Result<Gaussian3Mesh, Gaussian3Error> {
        if latitude_segments < 2 || longitude_segments < 3 {
            return Err(Gaussian3Error::InvalidMeshResolution);
        }
        let radius = confidence_radius_3d(confidence)?;
        let lower = self
            .covariance
            .cholesky_lower()
            .ok_or(Gaussian3Error::SingularCovariance)?;

        let mut vertices = Vec::with_capacity((latitude_segments + 1) * longitude_segments);
        let mut normals = Vec::with_capacity((latitude_segments + 1) * longitude_segments);
        for lat in 0..=latitude_segments {
            let theta = core::f64::consts::PI * lat as f64 / latitude_segments as f64;
            let sin_theta = theta.sin();
            let cos_theta = theta.cos();
            for lon in 0..longitude_segments {
                let phi = TWO_PI * lon as f64 / longitude_segments as f64;
                let unit = Vec3::new(sin_theta * phi.cos(), sin_theta * phi.sin(), cos_theta);
                let vertex = self.mean.add(lower.mul_vec(unit).scale(radius));
                vertices.push(vertex);
                normals.push(
                    self.covariance_inverse
                        .mul_vec(vertex.sub(self.mean))
                        .normalize(),
                );
            }
        }

        let mut triangles = Vec::with_capacity(latitude_segments * longitude_segments * 2);
        for lat in 0..latitude_segments {
            for lon in 0..longitude_segments {
                let next_lon = (lon + 1) % longitude_segments;
                let a = (lat * longitude_segments + lon) as u32;
                let b = ((lat + 1) * longitude_segments + lon) as u32;
                let c = ((lat + 1) * longitude_segments + next_lon) as u32;
                let d = (lat * longitude_segments + next_lon) as u32;
                if lat > 0 {
                    triangles.push([a, b, d]);
                }
                if lat + 1 < latitude_segments {
                    triangles.push([d, b, c]);
                }
            }
        }

        Ok(Gaussian3Mesh {
            confidence,
            radius,
            vertices,
            normals,
            triangles,
        })
    }
}

/// Geometric descriptor of a Gaussian confidence ellipsoid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gaussian3Ellipsoid {
    /// Ellipsoid center.
    pub center: Vec3,
    /// Confidence mass used to scale the ellipsoid.
    pub confidence: f64,
    /// Mahalanobis radius for this confidence mass.
    pub mahalanobis_radius: f64,
    /// Principal covariance axes.
    pub axes: SymmetricEigen3,
    /// Semi-axis radii in the same order as `axes`.
    pub radii: [f64; 3],
    /// Ellipsoid volume.
    pub volume: f64,
}

/// Compact diagnostics for a fitted Gaussian.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gaussian3Summary {
    /// Fitted mean.
    pub mean: Vec3,
    /// Covariance determinant.
    pub determinant: f64,
    /// Differential entropy in nats.
    pub entropy: f64,
    /// Covariance condition number.
    pub condition_number: f64,
    /// Fraction of variance explained by each principal axis.
    pub variance_explained: [f64; 3],
    /// Confidence ellipsoid volume for the requested confidence.
    pub ellipsoid_volume: f64,
}

/// One sigma point for uncertainty visualization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gaussian3SigmaPoint {
    /// Sigma point position.
    pub point: Vec3,
    /// Principal axis index, or `None` for the center point.
    pub axis: Option<usize>,
    /// Axis sign: `-1`, `0`, or `1`.
    pub sign: i8,
}

/// Sigma point set for one Gaussian model.
#[derive(Debug, Clone, PartialEq)]
pub struct Gaussian3SigmaPointSet {
    /// Confidence used to scale the sigma shell.
    pub confidence: f64,
    /// Mahalanobis radius for this confidence.
    pub radius: f64,
    /// Center plus paired axis endpoints.
    pub points: Vec<Gaussian3SigmaPoint>,
}

/// Feature axis in the 3D cooperative game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gaussian3Feature {
    /// X coordinate.
    X,
    /// Y coordinate.
    Y,
    /// Z coordinate.
    Z,
}

impl Gaussian3Feature {
    /// Feature order used by attribution arrays.
    pub const ALL: [Self; 3] = [Self::X, Self::Y, Self::Z];

    /// Zero-based feature index.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }

    /// Short stable feature name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::X => "x",
            Self::Y => "y",
            Self::Z => "z",
        }
    }
}

/// Scalar payoff used by SHAP/game-theory attribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gaussian3ShapleyScore {
    /// Squared Mahalanobis distance; additive for diagonal covariance.
    MahalanobisSquared,
    /// Negative log density; interpretable as anomaly surprisal in nats.
    Surprisal,
    /// Natural log density.
    LogPdf,
    /// Probability density.
    Pdf,
    /// Cumulative confidence mass at the point.
    Confidence,
    /// Upper-tail probability beyond the point.
    TailProbability,
}

/// Cooperative feature game for a 3D Gaussian payoff.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gaussian3FeatureGame {
    /// Gaussian model being explained.
    pub model: Gaussian3,
    /// Foreground point being explained.
    pub point: Vec3,
    /// Background point used for missing features.
    pub background: Vec3,
    /// Scalar payoff.
    pub score: Gaussian3ShapleyScore,
}

impl Gaussian3FeatureGame {
    /// Build a feature game.
    #[must_use]
    pub const fn new(
        model: Gaussian3,
        point: Vec3,
        background: Vec3,
        score: Gaussian3ShapleyScore,
    ) -> Self {
        Self {
            model,
            point,
            background,
            score,
        }
    }

    /// Coalition payoff for a bitmask over `{x, y, z}`.
    #[must_use]
    pub fn payoff(&self, coalition_mask: u8) -> f64 {
        let point = coalition_point(self.point, self.background, coalition_mask);
        score_point(&self.model, point, self.score)
    }

    /// Exact Shapley values over the three-feature game.
    #[must_use]
    pub fn shapley_values(&self) -> Gaussian3Shapley {
        let coalition_values = coalition_values(self);
        let mut values = [0.0; 3];
        for (feature, value) in values.iter_mut().enumerate() {
            for coalition in 0..8u8 {
                if coalition & feature_mask(feature) != 0 {
                    continue;
                }
                let size = coalition.count_ones() as usize;
                let weight = shapley_weight(size);
                let with_feature = coalition | feature_mask(feature);
                *value += weight
                    * (coalition_values[with_feature as usize]
                        - coalition_values[coalition as usize]);
            }
        }

        Gaussian3Shapley {
            score: self.score,
            point: self.point,
            background: self.background,
            baseline: coalition_values[0],
            prediction: coalition_values[7],
            values,
        }
    }

    /// Exact Shapley interaction matrix.
    #[must_use]
    pub fn shapley_interactions(&self) -> Gaussian3ShapleyInteraction {
        let shapley = self.shapley_values();
        let coalition_values = coalition_values(self);
        let mut interactions = [[0.0; 3]; 3];

        for i in [0, 1, 2] {
            for j in (i + 1)..3 {
                let mut value = 0.0;
                for coalition in 0..8u8 {
                    let pair = feature_mask(i) | feature_mask(j);
                    if coalition & pair != 0 {
                        continue;
                    }
                    let size = coalition.count_ones() as usize;
                    let delta = coalition_values[(coalition | pair) as usize]
                        - coalition_values[(coalition | feature_mask(i)) as usize]
                        - coalition_values[(coalition | feature_mask(j)) as usize]
                        + coalition_values[coalition as usize];
                    value += shapley_interaction_weight(size) * delta;
                }
                interactions[i][j] = value;
                interactions[j][i] = value;
            }
        }

        for (i, row) in interactions.iter_mut().enumerate() {
            let off_diagonal_sum = row
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, value)| *value)
                .sum::<f64>();
            row[i] = shapley.values[i] - off_diagonal_sum;
        }

        Gaussian3ShapleyInteraction {
            score: self.score,
            point: self.point,
            background: self.background,
            baseline: shapley.baseline,
            prediction: shapley.prediction,
            shapley_values: shapley.values,
            values: interactions,
        }
    }
}

/// Exact SHAP values for a 3D Gaussian feature game.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gaussian3Shapley {
    /// Scalar payoff explained by the values.
    pub score: Gaussian3ShapleyScore,
    /// Foreground point.
    pub point: Vec3,
    /// Background point.
    pub background: Vec3,
    /// Payoff with no active features.
    pub baseline: f64,
    /// Payoff with all features active.
    pub prediction: f64,
    /// Feature attributions ordered `[x, y, z]`.
    pub values: [f64; 3],
}

impl Gaussian3Shapley {
    /// Sum of feature attributions.
    #[must_use]
    pub fn attribution_sum(&self) -> f64 {
        self.values.iter().sum()
    }

    /// `baseline + sum(values)`, which should match `prediction`.
    #[must_use]
    pub fn reconstructed_prediction(&self) -> f64 {
        self.baseline + self.attribution_sum()
    }

    /// Attribution for a named feature.
    #[must_use]
    pub fn value_for(&self, feature: Gaussian3Feature) -> f64 {
        self.values[feature.index()]
    }
}

/// Exact SHAP interaction matrix for a 3D Gaussian feature game.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gaussian3ShapleyInteraction {
    /// Scalar payoff explained by the matrix.
    pub score: Gaussian3ShapleyScore,
    /// Foreground point.
    pub point: Vec3,
    /// Background point.
    pub background: Vec3,
    /// Payoff with no active features.
    pub baseline: f64,
    /// Payoff with all features active.
    pub prediction: f64,
    /// SHAP values ordered `[x, y, z]`.
    pub shapley_values: [f64; 3],
    /// Interaction matrix. Rows sum to `shapley_values`.
    pub values: [[f64; 3]; 3],
}

impl Gaussian3ShapleyInteraction {
    /// Row sum for one feature.
    #[must_use]
    pub fn row_sum(&self, feature: Gaussian3Feature) -> f64 {
        self.values[feature.index()].iter().sum()
    }

    /// Pairwise interaction between two features.
    #[must_use]
    pub fn interaction(&self, left: Gaussian3Feature, right: Gaussian3Feature) -> f64 {
        self.values[left.index()][right.index()]
    }
}

/// Axis-aligned 3D bounds for density-volume generation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds3 {
    /// Minimum corner.
    pub min: Vec3,
    /// Maximum corner.
    pub max: Vec3,
}

impl Bounds3 {
    /// Build 3D bounds.
    #[must_use]
    pub const fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    fn is_valid(self) -> bool {
        self.min.is_finite()
            && self.max.is_finite()
            && self.min.x <= self.max.x
            && self.min.y <= self.max.y
            && self.min.z <= self.max.z
    }
}

/// Density evaluated at one grid position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gaussian3DensityVoxel {
    /// Grid position.
    pub position: Vec3,
    /// Probability density.
    pub density: f64,
}

/// Triangle mesh representation of a Gaussian confidence ellipsoid.
#[derive(Debug, Clone, PartialEq)]
pub struct Gaussian3Mesh {
    /// Confidence mass used to scale the ellipsoid.
    pub confidence: f64,
    /// Mahalanobis radius of the ellipsoid.
    pub radius: f64,
    /// 3D vertex positions.
    pub vertices: Vec<Vec3>,
    /// Unit normals matching `vertices`.
    pub normals: Vec<Vec3>,
    /// Triangle vertex indices.
    pub triangles: Vec<[u32; 3]>,
}

/// One weighted component in a Gaussian mixture model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gaussian3Component {
    /// Mixture weight. All component weights sum to 1.
    pub weight: f64,
    /// Component distribution.
    pub model: Gaussian3,
}

/// Options for deterministic EM fitting of a 3D Gaussian mixture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gaussian3MixtureFitOptions {
    /// Number of mixture components.
    pub components: usize,
    /// Maximum EM iterations.
    pub max_iterations: usize,
    /// Relative log-likelihood convergence tolerance.
    pub tolerance: f64,
    /// Non-negative amount added to each component covariance diagonal.
    pub regularization: f64,
    /// Components below this effective sample fraction are stabilized.
    pub min_component_weight: f64,
}

impl Default for Gaussian3MixtureFitOptions {
    fn default() -> Self {
        Self {
            components: 2,
            max_iterations: 100,
            tolerance: 1.0e-6,
            regularization: 1.0e-6,
            min_component_weight: 1.0e-4,
        }
    }
}

/// A full-covariance Gaussian mixture fitted in 3D.
#[derive(Debug, Clone, PartialEq)]
pub struct Gaussian3Mixture {
    /// Fitted weighted components.
    pub components: Vec<Gaussian3Component>,
    /// Final training-set log likelihood.
    pub log_likelihood: f64,
    /// Number of EM iterations run.
    pub iterations: usize,
    /// True when the relative likelihood tolerance was reached.
    pub converged: bool,
}

impl Gaussian3Mixture {
    /// Fit a deterministic full-covariance Gaussian mixture with EM.
    pub fn fit(
        samples: &[Vec3],
        options: Gaussian3MixtureFitOptions,
    ) -> Result<Self, Gaussian3Error> {
        validate_mixture_options(samples, options)?;
        for &sample in samples {
            if !sample.is_finite() {
                return Err(Gaussian3Error::NonFiniteSample(sample));
            }
        }

        let common_covariance = regularized_covariance(
            samples,
            mean_of(samples),
            samples.len() as f64,
            options.regularization.max(1.0e-12),
        );
        let means = deterministic_initial_means(samples, options.components);
        let mut components: Vec<Gaussian3Component> = means
            .into_iter()
            .map(|mean| {
                Gaussian3::from_mean_covariance(mean, common_covariance).map(|model| {
                    Gaussian3Component {
                        weight: 1.0 / options.components as f64,
                        model,
                    }
                })
            })
            .collect::<Result<_, _>>()?;

        let mut log_likelihood = mixture_log_likelihood(&components, samples);
        let mut converged = false;
        let mut iterations = 0;

        for iteration in 1..=options.max_iterations {
            iterations = iteration;
            let responsibilities = expectation(&components, samples);
            let next_components = maximize(
                samples,
                &responsibilities,
                &components,
                &common_covariance,
                options,
            )?;
            let next_log_likelihood = mixture_log_likelihood(&next_components, samples);
            let scale = 1.0 + log_likelihood.abs();
            let improvement = next_log_likelihood - log_likelihood;

            components = next_components;
            log_likelihood = next_log_likelihood;

            if improvement.abs() <= options.tolerance * scale {
                converged = true;
                break;
            }
        }

        Ok(Self {
            components,
            log_likelihood,
            iterations,
            converged,
        })
    }

    /// Fit a Gaussian mixture from three numeric dataset columns.
    pub fn fit_dataset(
        dataset: &Dataset,
        x: &str,
        y: &str,
        z: &str,
        options: Gaussian3MixtureFitOptions,
    ) -> Result<Self, Gaussian3Error> {
        let samples = samples_from_dataset(dataset, x, y, z)?;
        Self::fit(&samples, options)
    }

    /// Fit several component counts and return the model with lowest BIC.
    pub fn fit_best_bic<I>(
        samples: &[Vec3],
        component_counts: I,
        base_options: Gaussian3MixtureFitOptions,
    ) -> Result<Gaussian3MixtureSelection, Gaussian3Error>
    where
        I: IntoIterator<Item = usize>,
    {
        let mut best: Option<Gaussian3Mixture> = None;
        let mut candidates = Vec::new();

        for components in component_counts {
            let mut options = base_options;
            options.components = components;
            let model = Self::fit(samples, options)?;
            let candidate = Gaussian3MixtureCandidate {
                components,
                log_likelihood: model.log_likelihood,
                aic: model.aic(),
                bic: model.bic(samples.len()),
                iterations: model.iterations,
                converged: model.converged,
            };

            let is_better = best
                .as_ref()
                .map(|current| candidate.bic < current.bic(samples.len()))
                .unwrap_or(true);
            if is_better {
                best = Some(model);
            }
            candidates.push(candidate);
        }

        let best = best.ok_or(Gaussian3Error::EmptyModelSelection)?;
        Ok(Gaussian3MixtureSelection { best, candidates })
    }

    /// Fit several component counts from dataset columns and choose by BIC.
    pub fn fit_dataset_best_bic<I>(
        dataset: &Dataset,
        x: &str,
        y: &str,
        z: &str,
        component_counts: I,
        base_options: Gaussian3MixtureFitOptions,
    ) -> Result<Gaussian3MixtureSelection, Gaussian3Error>
    where
        I: IntoIterator<Item = usize>,
    {
        let samples = samples_from_dataset(dataset, x, y, z)?;
        Self::fit_best_bic(&samples, component_counts, base_options)
    }

    /// Mixture density at a point.
    #[must_use]
    pub fn pdf(&self, point: Vec3) -> f64 {
        self.components
            .iter()
            .map(|component| component.weight * component.model.pdf(point))
            .sum()
    }

    /// Natural log mixture density at a point.
    #[must_use]
    pub fn log_pdf(&self, point: Vec3) -> f64 {
        let logs: Vec<f64> = self
            .components
            .iter()
            .map(|component| component.weight.ln() + component.model.log_pdf(point))
            .collect();
        log_sum_exp(&logs)
    }

    /// Posterior component probabilities for a point.
    #[must_use]
    pub fn responsibilities(&self, point: Vec3) -> Vec<f64> {
        let logs: Vec<f64> = self
            .components
            .iter()
            .map(|component| component.weight.ln() + component.model.log_pdf(point))
            .collect();
        let normalizer = log_sum_exp(&logs);
        logs.iter()
            .map(|value| (*value - normalizer).exp())
            .collect()
    }

    /// Most likely component index for a point.
    #[must_use]
    pub fn predict_component(&self, point: Vec3) -> usize {
        let responsibilities = self.responsibilities(point);
        responsibilities
            .iter()
            .enumerate()
            .max_by(|left, right| left.1.total_cmp(right.1))
            .map(|(index, _)| index)
            .unwrap_or(0)
    }

    /// Akaike information criterion for the fitted observations.
    #[must_use]
    pub fn aic(&self) -> f64 {
        2.0 * self.parameter_count() as f64 - 2.0 * self.log_likelihood
    }

    /// Bayesian information criterion for `sample_count` observations.
    #[must_use]
    pub fn bic(&self, sample_count: usize) -> f64 {
        self.parameter_count() as f64 * (sample_count as f64).ln() - 2.0 * self.log_likelihood
    }

    /// Number of free parameters in the mixture.
    #[must_use]
    pub fn parameter_count(&self) -> usize {
        // Per component: 3 means + 6 covariance terms. Weights add k - 1.
        self.components.len() * 9 + self.components.len().saturating_sub(1)
    }
}

/// BIC model-selection result for Gaussian mixtures.
#[derive(Debug, Clone, PartialEq)]
pub struct Gaussian3MixtureSelection {
    /// Model with the lowest BIC.
    pub best: Gaussian3Mixture,
    /// Metadata for every fitted candidate.
    pub candidates: Vec<Gaussian3MixtureCandidate>,
}

/// One candidate evaluated during mixture model selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gaussian3MixtureCandidate {
    /// Component count.
    pub components: usize,
    /// Final log likelihood.
    pub log_likelihood: f64,
    /// Akaike information criterion.
    pub aic: f64,
    /// Bayesian information criterion.
    pub bic: f64,
    /// EM iterations used.
    pub iterations: usize,
    /// True when the candidate converged.
    pub converged: bool,
}

/// Errors returned by 3D Gaussian fitting and modeling helpers.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum Gaussian3Error {
    /// At least two samples are needed for a full covariance estimate.
    #[error("at least two samples are required, got {actual}")]
    InsufficientSamples {
        /// Actual sample count.
        actual: usize,
    },
    /// Sample contained a non-finite coordinate.
    #[error("sample contains a non-finite coordinate: {0:?}")]
    NonFiniteSample(Vec3),
    /// Mean contained a non-finite coordinate.
    #[error("mean contains a non-finite coordinate: {0:?}")]
    NonFiniteMean(Vec3),
    /// Covariance contained a non-finite value.
    #[error("covariance contains a non-finite value")]
    NonFiniteCovariance,
    /// Covariance was not symmetric.
    #[error("covariance must be symmetric")]
    NonsymmetricCovariance,
    /// Covariance is not positive definite.
    #[error("covariance must be positive definite")]
    SingularCovariance,
    /// Regularization must be finite and non-negative.
    #[error("regularization must be finite and non-negative, got {0}")]
    InvalidRegularization(f64),
    /// Dataset column was missing.
    #[error("missing dataset column `{0}`")]
    MissingColumn(String),
    /// Dataset column could not be read as numeric values.
    #[error("dataset column `{0}` must be numeric")]
    NonNumericColumn(String),
    /// Confidence must be in `(0, 1)`.
    #[error("confidence must be in (0, 1), got {0}")]
    InvalidConfidence(f64),
    /// Density bounds are malformed.
    #[error("bounds must be finite and ordered")]
    InvalidBounds,
    /// Density grid dimensions must be non-zero.
    #[error("grid steps must all be >= 1")]
    InvalidGrid,
    /// Mesh resolution is too low to form triangles.
    #[error("mesh requires at least 2 latitude and 3 longitude segments")]
    InvalidMeshResolution,
    /// Mixture component count was incompatible with the sample count.
    #[error("component count must be in 1..={samples}, got {components}")]
    InvalidComponentCount {
        /// Requested components.
        components: usize,
        /// Available samples.
        samples: usize,
    },
    /// EM iteration count must be positive.
    #[error("max_iterations must be >= 1")]
    InvalidIterations,
    /// EM tolerance must be finite and non-negative.
    #[error("tolerance must be finite and non-negative, got {0}")]
    InvalidTolerance(f64),
    /// Minimum component weight must be finite and in `[0, 1)`.
    #[error("min_component_weight must be finite and in [0, 1), got {0}")]
    InvalidComponentWeight(f64),
    /// No candidate component counts were supplied.
    #[error("at least one model-selection candidate is required")]
    EmptyModelSelection,
}

fn samples_from_dataset(
    dataset: &Dataset,
    x: &str,
    y: &str,
    z: &str,
) -> Result<Vec<Vec3>, Gaussian3Error> {
    let x_col = dataset
        .column(x)
        .ok_or_else(|| Gaussian3Error::MissingColumn(x.to_string()))?;
    let y_col = dataset
        .column(y)
        .ok_or_else(|| Gaussian3Error::MissingColumn(y.to_string()))?;
    let z_col = dataset
        .column(z)
        .ok_or_else(|| Gaussian3Error::MissingColumn(z.to_string()))?;

    let mut samples = Vec::with_capacity(dataset.len());
    for row in 0..dataset.len() {
        let x_value = x_col
            .read_f64(row)
            .ok_or_else(|| Gaussian3Error::NonNumericColumn(x.to_string()))?;
        let y_value = y_col
            .read_f64(row)
            .ok_or_else(|| Gaussian3Error::NonNumericColumn(y.to_string()))?;
        let z_value = z_col
            .read_f64(row)
            .ok_or_else(|| Gaussian3Error::NonNumericColumn(z.to_string()))?;
        samples.push(Vec3::new(x_value, y_value, z_value));
    }
    Ok(samples)
}

fn largest_off_diagonal(rows: [[f64; 3]; 3]) -> (usize, usize, f64) {
    let candidates = [
        (0, 1, rows[0][1].abs()),
        (0, 2, rows[0][2].abs()),
        (1, 2, rows[1][2].abs()),
    ];
    candidates
        .into_iter()
        .max_by(|left, right| left.2.total_cmp(&right.2))
        .unwrap()
}

fn validate_mixture_options(
    samples: &[Vec3],
    options: Gaussian3MixtureFitOptions,
) -> Result<(), Gaussian3Error> {
    if samples.len() < 2 {
        return Err(Gaussian3Error::InsufficientSamples {
            actual: samples.len(),
        });
    }
    if options.components == 0 || options.components > samples.len() {
        return Err(Gaussian3Error::InvalidComponentCount {
            components: options.components,
            samples: samples.len(),
        });
    }
    if options.max_iterations == 0 {
        return Err(Gaussian3Error::InvalidIterations);
    }
    if options.tolerance < 0.0 || !options.tolerance.is_finite() {
        return Err(Gaussian3Error::InvalidTolerance(options.tolerance));
    }
    if options.regularization < 0.0 || !options.regularization.is_finite() {
        return Err(Gaussian3Error::InvalidRegularization(
            options.regularization,
        ));
    }
    if options.min_component_weight < 0.0
        || options.min_component_weight >= 1.0
        || !options.min_component_weight.is_finite()
    {
        return Err(Gaussian3Error::InvalidComponentWeight(
            options.min_component_weight,
        ));
    }
    Ok(())
}

fn mean_of(samples: &[Vec3]) -> Vec3 {
    samples
        .iter()
        .copied()
        .fold(Vec3::default(), Vec3::add)
        .scale(1.0 / samples.len() as f64)
}

fn regularized_covariance(
    samples: &[Vec3],
    mean: Vec3,
    denominator: f64,
    regularization: f64,
) -> Mat3 {
    let mut rows = [[0.0; 3]; 3];
    for &sample in samples {
        accumulate_outer(&mut rows, sample.sub(mean), 1.0);
    }
    for row in &mut rows {
        for value in row {
            *value /= denominator;
        }
    }
    Mat3::new(rows).add_diagonal(regularization)
}

fn weighted_gaussian(
    samples: &[Vec3],
    weights: &[f64],
    weight_sum: f64,
    regularization: f64,
) -> Result<Gaussian3, Gaussian3Error> {
    let mut mean = Vec3::default();
    for (&sample, &weight) in samples.iter().zip(weights) {
        mean = mean.add(sample.scale(weight));
    }
    mean = mean.scale(1.0 / weight_sum);

    let mut covariance = [[0.0; 3]; 3];
    for (&sample, &weight) in samples.iter().zip(weights) {
        accumulate_outer(&mut covariance, sample.sub(mean), weight);
    }
    for row in &mut covariance {
        for value in row {
            *value /= weight_sum;
        }
    }

    Gaussian3::from_mean_covariance(mean, Mat3::new(covariance).add_diagonal(regularization))
}

fn accumulate_outer(rows: &mut [[f64; 3]; 3], value: Vec3, weight: f64) {
    let values = [value.x, value.y, value.z];
    for row in 0..3 {
        for col in 0..3 {
            rows[row][col] += weight * values[row] * values[col];
        }
    }
}

fn deterministic_initial_means(samples: &[Vec3], components: usize) -> Vec<Vec3> {
    let global_mean = mean_of(samples);
    let first = samples
        .iter()
        .copied()
        .min_by(|left, right| {
            squared_distance(*left, global_mean).total_cmp(&squared_distance(*right, global_mean))
        })
        .unwrap_or(global_mean);

    let mut means = vec![first];
    while means.len() < components {
        let next = samples
            .iter()
            .copied()
            .max_by(|left, right| {
                nearest_distance_squared(*left, &means)
                    .total_cmp(&nearest_distance_squared(*right, &means))
            })
            .unwrap_or(first);
        means.push(next);
    }
    means
}

fn nearest_distance_squared(sample: Vec3, means: &[Vec3]) -> f64 {
    means
        .iter()
        .map(|&mean| squared_distance(sample, mean))
        .fold(f64::INFINITY, f64::min)
}

fn squared_distance(left: Vec3, right: Vec3) -> f64 {
    let delta = left.sub(right);
    delta.dot(delta)
}

fn expectation(components: &[Gaussian3Component], samples: &[Vec3]) -> Vec<f64> {
    let k = components.len();
    let mut responsibilities = vec![0.0; samples.len() * k];
    let mut logs = vec![0.0; k];

    for (row, &sample) in samples.iter().enumerate() {
        for (component_index, component) in components.iter().enumerate() {
            logs[component_index] = component.weight.ln() + component.model.log_pdf(sample);
        }
        let normalizer = log_sum_exp(&logs);
        for component_index in 0..k {
            responsibilities[row * k + component_index] =
                (logs[component_index] - normalizer).exp();
        }
    }

    responsibilities
}

fn maximize(
    samples: &[Vec3],
    responsibilities: &[f64],
    previous: &[Gaussian3Component],
    fallback_covariance: &Mat3,
    options: Gaussian3MixtureFitOptions,
) -> Result<Vec<Gaussian3Component>, Gaussian3Error> {
    let k = previous.len();
    let n = samples.len();
    let floor_weight = options.min_component_weight.max(1.0e-12);
    let mut next = Vec::with_capacity(k);

    for component_index in 0..k {
        let weights: Vec<f64> = (0..n)
            .map(|row| responsibilities[row * k + component_index])
            .collect();
        let weight_sum = weights.iter().sum::<f64>();

        if weight_sum <= floor_weight * n as f64 {
            next.push(Gaussian3Component {
                weight: floor_weight,
                model: Gaussian3::from_mean_covariance(
                    previous[component_index].model.mean,
                    *fallback_covariance,
                )?,
            });
            continue;
        }

        let model = weighted_gaussian(samples, &weights, weight_sum, options.regularization)?;
        next.push(Gaussian3Component {
            weight: weight_sum / n as f64,
            model,
        });
    }

    let weight_total = next.iter().map(|component| component.weight).sum::<f64>();
    for component in &mut next {
        component.weight /= weight_total;
    }

    Ok(next)
}

fn mixture_log_likelihood(components: &[Gaussian3Component], samples: &[Vec3]) -> f64 {
    let mut logs = vec![0.0; components.len()];
    samples
        .iter()
        .map(|&sample| {
            for (index, component) in components.iter().enumerate() {
                logs[index] = component.weight.ln() + component.model.log_pdf(sample);
            }
            log_sum_exp(&logs)
        })
        .sum()
}

fn log_sum_exp(values: &[f64]) -> f64 {
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !max.is_finite() {
        return max;
    }
    max + values
        .iter()
        .map(|value| (*value - max).exp())
        .sum::<f64>()
        .ln()
}

fn coalition_values(game: &Gaussian3FeatureGame) -> [f64; 8] {
    let mut values = [0.0; 8];
    for coalition in 0..8u8 {
        values[coalition as usize] = game.payoff(coalition);
    }
    values
}

fn coalition_point(point: Vec3, background: Vec3, coalition_mask: u8) -> Vec3 {
    Vec3::new(
        if coalition_mask & feature_mask(0) != 0 {
            point.x
        } else {
            background.x
        },
        if coalition_mask & feature_mask(1) != 0 {
            point.y
        } else {
            background.y
        },
        if coalition_mask & feature_mask(2) != 0 {
            point.z
        } else {
            background.z
        },
    )
}

fn score_point(model: &Gaussian3, point: Vec3, score: Gaussian3ShapleyScore) -> f64 {
    match score {
        Gaussian3ShapleyScore::MahalanobisSquared => model.mahalanobis_squared(point),
        Gaussian3ShapleyScore::Surprisal => model.surprisal(point),
        Gaussian3ShapleyScore::LogPdf => model.log_pdf(point),
        Gaussian3ShapleyScore::Pdf => model.pdf(point),
        Gaussian3ShapleyScore::Confidence => model.confidence_of(point),
        Gaussian3ShapleyScore::TailProbability => model.tail_probability(point),
    }
}

const fn feature_mask(feature: usize) -> u8 {
    1 << feature
}

fn shapley_weight(coalition_size_without_feature: usize) -> f64 {
    match coalition_size_without_feature {
        0 => 1.0 / 3.0,
        1 => 1.0 / 6.0,
        2 => 1.0 / 3.0,
        _ => 0.0,
    }
}

fn shapley_interaction_weight(coalition_size_without_pair: usize) -> f64 {
    match coalition_size_without_pair {
        0 | 1 => 0.25,
        _ => 0.0,
    }
}

fn trace_product(left: Mat3, right: Mat3) -> f64 {
    let mut trace = 0.0;
    for row in 0..3 {
        for col in 0..3 {
            trace += left.rows[row][col] * right.rows[col][row];
        }
    }
    trace
}

fn average_covariance(left: Mat3, right: Mat3) -> Mat3 {
    let mut rows = [[0.0; 3]; 3];
    for (row_index, row) in rows.iter_mut().enumerate() {
        for (col_index, value) in row.iter_mut().enumerate() {
            *value = 0.5 * (left.rows[row_index][col_index] + right.rows[row_index][col_index]);
        }
    }
    Mat3::new(rows)
}

fn chi_square3_cdf(x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if !x.is_finite() {
        return 1.0;
    }
    let root = (0.5 * x).sqrt();
    (erf(root) - (2.0 * x / core::f64::consts::PI).sqrt() * (-0.5 * x).exp()).clamp(0.0, 1.0)
}

fn erf(x: f64) -> f64 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + 0.327_591_1 * x);
    let y = 1.0
        - (((((1.061_405_429 * t - 1.453_152_027) * t + 1.421_413_741) * t - 0.284_496_736) * t
            + 0.254_829_592)
            * t
            * (-x * x).exp());
    sign * y
}

/// Approximate Mahalanobis radius for a 3D confidence ellipsoid.
///
/// This is `sqrt(chi_square_quantile(confidence, df=3))`, using the
/// Wilson-Hilferty transform. The approximation is tight enough for visual
/// confidence surfaces and avoids adding a special-function dependency.
pub fn confidence_radius_3d(confidence: f64) -> Result<f64, Gaussian3Error> {
    if !(0.0..1.0).contains(&confidence) || !confidence.is_finite() {
        return Err(Gaussian3Error::InvalidConfidence(confidence));
    }
    let p = confidence.clamp(MIN_CONFIDENCE, MAX_CONFIDENCE);
    let z = inverse_standard_normal_cdf(p);
    let k: f64 = 3.0;
    let q = k * (1.0 - 2.0 / (9.0 * k) + z * (2.0 / (9.0 * k)).sqrt()).powi(3);
    Ok(q.max(0.0).sqrt())
}

fn grid_value(min: f64, max: f64, index: usize, steps: usize) -> f64 {
    if steps == 1 {
        return (min + max) * 0.5;
    }
    min + (max - min) * index as f64 / (steps - 1) as f64
}

// Peter John Acklam's inverse-normal approximation, arranged as public-domain
// coefficients. Relative error is more than adequate for confidence surfaces.
fn inverse_standard_normal_cdf(p: f64) -> f64 {
    const A: [f64; 6] = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_69e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    const D: [f64; 4] = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
    ];

    let plow = 0.02425;
    let phigh = 1.0 - plow;

    if p < plow {
        let q = (-2.0 * p.ln()).sqrt();
        return (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0);
    }
    if p > phigh {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        return -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0);
    }

    let q = p - 0.5;
    let r = q * q;
    (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
        / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use berthacharts_core::{Column, ColumnData, DatasetId};

    fn assert_near(left: f64, right: f64, tolerance: f64) {
        assert!(
            (left - right).abs() <= tolerance,
            "{left} should be within {tolerance} of {right}"
        );
    }

    #[test]
    fn fits_mean_and_covariance_from_samples() {
        let samples = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::new(0.0, 0.0, 2.0),
        ];
        let model = Gaussian3::fit(
            &samples,
            Gaussian3FitOptions {
                estimator: CovarianceEstimator::MaximumLikelihood,
                regularization: 0.25,
            },
        )
        .unwrap();

        assert_eq!(model.mean, Vec3::new(0.5, 0.5, 0.5));
        assert_near(model.covariance.rows[0][0], 0.75 + 0.25, 1.0e-12);
        assert_near(model.covariance.rows[1][1], 0.75 + 0.25, 1.0e-12);
        assert_near(model.covariance.rows[2][2], 0.75 + 0.25, 1.0e-12);
        assert_near(model.covariance.rows[0][1], -0.25, 1.0e-12);
    }

    #[test]
    fn standard_normal_density_matches_known_center_value() {
        let model = Gaussian3::from_mean_covariance(
            Vec3::new(0.0, 0.0, 0.0),
            Mat3::diagonal(1.0, 1.0, 1.0),
        )
        .unwrap();
        let expected = 1.0 / TWO_PI.powf(1.5);
        assert_near(model.pdf(Vec3::new(0.0, 0.0, 0.0)), expected, 1.0e-12);
        assert_near(
            model.mahalanobis_squared(Vec3::new(1.0, 2.0, 2.0)),
            9.0,
            1.0e-12,
        );
    }

    #[test]
    fn fits_from_numeric_dataset_columns() {
        let dataset = Dataset::new(
            DatasetId::new(7),
            0,
            vec![
                (
                    "x".into(),
                    Column::F64(ColumnData::new(vec![0.0, 2.0, 0.0, 0.0])),
                ),
                (
                    "y".into(),
                    Column::F64(ColumnData::new(vec![0.0, 0.0, 2.0, 0.0])),
                ),
                (
                    "z".into(),
                    Column::F64(ColumnData::new(vec![0.0, 0.0, 0.0, 2.0])),
                ),
            ],
        );

        let model = Gaussian3::fit_dataset(
            &dataset,
            "x",
            "y",
            "z",
            Gaussian3FitOptions {
                estimator: CovarianceEstimator::MaximumLikelihood,
                regularization: 0.25,
            },
        )
        .unwrap();

        assert_eq!(model.mean, Vec3::new(0.5, 0.5, 0.5));
    }

    #[test]
    fn confidence_checks_use_mahalanobis_radius() {
        let model = Gaussian3::from_mean_covariance(
            Vec3::new(0.0, 0.0, 0.0),
            Mat3::diagonal(1.0, 1.0, 1.0),
        )
        .unwrap();
        assert!(model
            .contains_confidence(Vec3::new(0.0, 0.0, 0.0), 0.95)
            .unwrap());
        assert!(!model
            .contains_confidence(Vec3::new(5.0, 0.0, 0.0), 0.95)
            .unwrap());
    }

    #[test]
    fn density_grid_covers_requested_points() {
        let model = Gaussian3::from_mean_covariance(
            Vec3::new(0.0, 0.0, 0.0),
            Mat3::diagonal(1.0, 1.0, 1.0),
        )
        .unwrap();
        let grid = model
            .density_grid(
                Bounds3::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0)),
                [3, 3, 3],
            )
            .unwrap();
        assert_eq!(grid.len(), 27);
        assert_eq!(grid[13].position, Vec3::new(0.0, 0.0, 0.0));
        assert!(grid[13].density > grid[0].density);
    }

    #[test]
    fn ellipsoid_mesh_has_expected_topology() {
        let model = Gaussian3::from_mean_covariance(
            Vec3::new(1.0, 2.0, 3.0),
            Mat3::diagonal(4.0, 1.0, 0.25),
        )
        .unwrap();
        let mesh = model.ellipsoid_mesh(0.90, 8, 12).unwrap();
        assert_eq!(mesh.vertices.len(), 9 * 12);
        assert_eq!(mesh.normals.len(), mesh.vertices.len());
        assert_eq!(mesh.triangles.len(), (8 - 1) * 12 * 2);
        assert!(mesh.radius > 0.0);
    }

    #[test]
    fn principal_axes_and_ellipsoid_descriptor_are_sorted() {
        let model = Gaussian3::from_mean_covariance(
            Vec3::new(1.0, 2.0, 3.0),
            Mat3::diagonal(9.0, 4.0, 1.0),
        )
        .unwrap();
        let axes = model.principal_axes().unwrap();
        assert_near(axes.values[0], 9.0, 1.0e-10);
        assert_near(axes.values[1], 4.0, 1.0e-10);
        assert_near(axes.values[2], 1.0, 1.0e-10);

        let ellipsoid = model.confidence_ellipsoid(0.95).unwrap();
        assert_eq!(ellipsoid.center, Vec3::new(1.0, 2.0, 3.0));
        assert!(ellipsoid.radii[0] > ellipsoid.radii[1]);
        assert!(ellipsoid.radii[1] > ellipsoid.radii[2]);
        assert!(ellipsoid.volume > 0.0);
    }

    #[test]
    fn diagnostics_distances_and_sigma_points_are_available() {
        let base = Gaussian3::from_mean_covariance(
            Vec3::new(0.0, 0.0, 0.0),
            Mat3::diagonal(4.0, 1.0, 0.25),
        )
        .unwrap();
        let shifted = Gaussian3::from_mean_covariance(
            Vec3::new(1.0, 0.0, 0.0),
            Mat3::diagonal(4.0, 1.0, 0.25),
        )
        .unwrap();

        assert_near(base.kl_divergence_to(&base), 0.0, 1.0e-12);
        assert!(base.kl_divergence_to(&shifted) > 0.0);
        assert_near(base.bhattacharyya_distance(&base).unwrap(), 0.0, 1.0e-12);
        assert!(base.bhattacharyya_distance(&shifted).unwrap() > 0.0);
        assert!(base.confidence_of(Vec3::new(4.0, 0.0, 0.0)) > 0.7);
        assert!(base.tail_probability(Vec3::new(8.0, 0.0, 0.0)) < 0.01);

        let summary = base.summary(0.90).unwrap();
        assert_near(summary.condition_number, 16.0, 1.0e-10);
        assert_near(summary.variance_explained.iter().sum(), 1.0, 1.0e-12);
        assert!(summary.entropy.is_finite());
        assert!(summary.ellipsoid_volume > 0.0);

        let sigma = base.sigma_points(0.90).unwrap();
        assert_eq!(sigma.points.len(), 7);
        assert_eq!(sigma.points[0].axis, None);
        assert!(
            sigma
                .points
                .iter()
                .filter(|point| point.axis == Some(0))
                .count()
                == 2
        );
    }

    #[test]
    fn shapley_values_are_exact_for_additive_mahalanobis_game() {
        let model = Gaussian3::from_mean_covariance(
            Vec3::new(0.0, 0.0, 0.0),
            Mat3::diagonal(4.0, 9.0, 16.0),
        )
        .unwrap();
        let shapley = model.shapley_values(
            Vec3::new(2.0, 3.0, 4.0),
            Gaussian3ShapleyScore::MahalanobisSquared,
        );

        assert_near(shapley.baseline, 0.0, 1.0e-12);
        assert_near(shapley.prediction, 3.0, 1.0e-12);
        assert_near(
            shapley.reconstructed_prediction(),
            shapley.prediction,
            1.0e-12,
        );
        assert_near(shapley.value_for(Gaussian3Feature::X), 1.0, 1.0e-12);
        assert_near(shapley.value_for(Gaussian3Feature::Y), 1.0, 1.0e-12);
        assert_near(shapley.value_for(Gaussian3Feature::Z), 1.0, 1.0e-12);

        let interactions = model.shapley_interactions(
            Vec3::new(2.0, 3.0, 4.0),
            Gaussian3ShapleyScore::MahalanobisSquared,
        );
        for feature in Gaussian3Feature::ALL {
            assert_near(
                interactions.row_sum(feature),
                shapley.value_for(feature),
                1.0e-12,
            );
        }
        assert_near(
            interactions.interaction(Gaussian3Feature::X, Gaussian3Feature::Y),
            0.0,
            1.0e-12,
        );
    }

    #[test]
    fn shapley_interactions_capture_correlated_features() {
        let model = Gaussian3::from_mean_covariance(
            Vec3::new(0.0, 0.0, 0.0),
            Mat3::new([[1.0, 0.5, 0.0], [0.5, 1.0, 0.0], [0.0, 0.0, 1.0]]),
        )
        .unwrap();
        let interactions = model.shapley_interactions(
            Vec3::new(1.0, 1.0, 0.0),
            Gaussian3ShapleyScore::MahalanobisSquared,
        );

        assert_near(interactions.prediction, 4.0 / 3.0, 1.0e-12);
        assert_near(
            interactions.interaction(Gaussian3Feature::X, Gaussian3Feature::Y),
            -2.0 / 3.0,
            1.0e-12,
        );
        assert_near(
            interactions.row_sum(Gaussian3Feature::X),
            interactions.shapley_values[0],
            1.0e-12,
        );
        assert_near(
            interactions.row_sum(Gaussian3Feature::Y),
            interactions.shapley_values[1],
            1.0e-12,
        );
    }

    #[test]
    fn mixture_model_separates_two_clusters() {
        let samples = [
            Vec3::new(-5.2, 0.0, 0.1),
            Vec3::new(-5.0, 0.2, -0.1),
            Vec3::new(-4.8, -0.1, 0.0),
            Vec3::new(-5.1, 0.1, 0.2),
            Vec3::new(4.8, 0.0, -0.1),
            Vec3::new(5.1, -0.2, 0.1),
            Vec3::new(5.3, 0.1, 0.0),
            Vec3::new(4.9, 0.2, 0.2),
        ];

        let mixture = Gaussian3Mixture::fit(
            &samples,
            Gaussian3MixtureFitOptions {
                components: 2,
                max_iterations: 80,
                tolerance: 1.0e-7,
                regularization: 0.05,
                min_component_weight: 1.0e-4,
            },
        )
        .unwrap();

        assert_eq!(mixture.components.len(), 2);
        assert!(mixture.converged);
        assert_near(
            mixture
                .components
                .iter()
                .map(|component| component.weight)
                .sum(),
            1.0,
            1.0e-12,
        );

        let left = mixture.responsibilities(Vec3::new(-5.0, 0.0, 0.0));
        let right = mixture.responsibilities(Vec3::new(5.0, 0.0, 0.0));
        assert_ne!(
            mixture.predict_component(Vec3::new(-5.0, 0.0, 0.0)),
            mixture.predict_component(Vec3::new(5.0, 0.0, 0.0))
        );
        assert!(left.iter().copied().fold(0.0, f64::max) > 0.95);
        assert!(right.iter().copied().fold(0.0, f64::max) > 0.95);
        assert_eq!(mixture.parameter_count(), 19);
        assert!(mixture.aic().is_finite());
        assert!(mixture.bic(samples.len()).is_finite());
    }

    #[test]
    fn mixture_model_selection_uses_lowest_bic() {
        let mut samples = Vec::new();
        for index in 0..20 {
            let offset = index as f64 * 0.01;
            samples.push(Vec3::new(-8.0 + offset, 0.2 - offset, offset));
            samples.push(Vec3::new(8.0 - offset, -0.2 + offset, -offset));
        }

        let selection = Gaussian3Mixture::fit_best_bic(
            &samples,
            1..=2,
            Gaussian3MixtureFitOptions {
                components: 1,
                max_iterations: 100,
                tolerance: 1.0e-7,
                regularization: 0.02,
                min_component_weight: 1.0e-4,
            },
        )
        .unwrap();

        assert_eq!(selection.candidates.len(), 2);
        assert_eq!(selection.best.components.len(), 2);
        assert!(selection.candidates[1].bic < selection.candidates[0].bic);
    }

    #[test]
    fn singular_covariance_errors() {
        let err = Gaussian3::from_mean_covariance(
            Vec3::new(0.0, 0.0, 0.0),
            Mat3::diagonal(1.0, 0.0, 1.0),
        )
        .unwrap_err();
        assert_eq!(err, Gaussian3Error::SingularCovariance);
    }

    #[test]
    fn nonsymmetric_covariance_errors() {
        let err = Gaussian3::from_mean_covariance(
            Vec3::new(0.0, 0.0, 0.0),
            Mat3::new([[1.0, 0.5, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]),
        )
        .unwrap_err();
        assert_eq!(err, Gaussian3Error::NonsymmetricCovariance);
    }
}
