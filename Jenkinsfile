pipeline {
  agent {
    label 'raven-windows'
  }
  stages {
    stage('build') {
      steps {
        script {
          sh 'cargo build && cargo build --release && cargo test'
        }
      }
    }
  }
  post {
    always {
      archiveArtifacts artifacts: 'target/release/ese_parser.exe', onlyIfSuccessful: true
    }
  }
}
