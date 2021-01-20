pipeline {
  agent {
    label 'asdf-fedora'
  }
  stages {
    stage('building') {
      steps {
        script {
          try {
            sh 'build/build-wheel-mingw.sh'
          }
          finally {
            sh 'docker image prune --force --filter "until=168h"'
          }
        }
      }
    }
  }
  post {
    always {
      archiveArtifacts artifacts: 'builds/*', onlyIfSuccessful: true
    }
  }
}
