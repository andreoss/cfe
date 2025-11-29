import { Builder, By, until } from 'selenium-webdriver'
import chrome from 'selenium-webdriver/chrome.js'
import { writeFileSync, mkdtempSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const baseUrl = process.env.BASE_URL ?? 'http://127.0.0.1:58081'
const chromedriverPath = process.env.CHROMEDRIVER_PATH
const apiUrl = process.env.API_URL ?? 'http://127.0.0.1:58080'

const PNG_BASE64 =
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAACklEQVR4nGP4DwABAQEAGdiK2wAAAABJRU5ErkJggg=='

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

async function buildDriver() {
  const options = new chrome.Options()
  options.addArguments('--headless=new', '--no-sandbox', '--disable-gpu')
  const builder = new Builder().forBrowser('chrome').setChromeOptions(options)
  if (chromedriverPath) {
    const service = new chrome.ServiceBuilder(chromedriverPath)
    builder.setChromeService(service)
  }
  return builder.build()
}

function writeFixtures() {
  const dir = mkdtempSync(join(tmpdir(), 'tcbs-avatar-'))
  const png = join(dir, 'avatar.png')
  writeFileSync(png, Buffer.from(PNG_BASE64, 'base64'))
  const svg = join(dir, 'sneaky.png')
  writeFileSync(
    svg,
    '<svg xmlns="http://www.w3.org/2000/svg"><script>alert(1)</script></svg>',
  )
  return { png, svg }
}

async function fetchStatus(driver, url) {
  return driver.executeAsyncScript(
    `const cb = arguments[arguments.length - 1];
     fetch(arguments[0], { cache: 'no-store' }).then(r => cb({ status: r.status, type: r.headers.get('content-type'), nosniff: r.headers.get('x-content-type-options') })).catch(() => cb({ status: 0 }));`,
    url,
  )
}

async function run() {
  const driver = await buildDriver()
  const suffix = Date.now().toString(36)
  const username = `e2e_av_${suffix}`
  const { png, svg } = writeFixtures()
  try {
    await driver.get(`${baseUrl}/register`)
    await driver.wait(until.elementLocated(By.name('username')), 5000)
    await driver.findElement(By.name('username')).sendKeys(username)
    await driver.findElement(By.name('email')).sendKeys(`${username}@example.com`)
    await driver.findElement(By.name('password')).sendKeys('correcthorse')
    await driver.findElement(By.css('button[type="submit"]')).click()
    await driver.wait(until.urlIs(`${baseUrl}/`), 5000)

    await driver.get(`${baseUrl}/u/${username}`)
    await driver.wait(until.elementLocated(By.name('avatar-file')), 5000)

    let probe = await fetchStatus(driver, `${apiUrl}/api/users/${username}/avatar`)
    assert(
      probe.status === 404 || probe.status === 0,
      `a new account should have no avatar, got ${probe.status}`,
    )

    await driver.findElement(By.name('avatar-file')).sendKeys(svg)
    await driver.findElement(By.xpath("//button[normalize-space(.)='Upload avatar']")).click()
    const alert = await driver.wait(until.elementLocated(By.css('[role="alert"]')), 5000)
    const alertText = await alert.getText()
    assert(
      alertText.toLowerCase().includes('unsupported'),
      `a script-carrying file must be refused, alert said: ${alertText}`,
    )

    await driver.findElement(By.name('avatar-file')).sendKeys(png)
    await driver.findElement(By.xpath("//button[normalize-space(.)='Upload avatar']")).click()
    await driver.wait(
      async () =>
        driver.executeScript(
          "const img = document.querySelector('img.avatar'); return img !== null && img.src.includes('?v=') && img.complete",
        ),
      10000,
      'the uploaded avatar should replace the placeholder and finish fetching',
    )

    const img = await driver.findElement(By.css('img.avatar'))
    const src = await img.getAttribute('src')
    assert(src.includes(`${apiUrl}/api/users/${username}/avatar`), 'the avatar should point at the api')
    const loaded = await driver.executeScript(
      'return arguments[0].complete && arguments[0].naturalWidth > 0',
      img,
    )
    assert(loaded === true, 'the uploaded avatar should actually render in the browser')

    const served = await fetchStatus(driver, src)
    assert(served.status === 200, `the avatar should be served, got ${served.status}`)
    assert(
      served.type && served.type.includes('image/png'),
      `the avatar should be served as png, got ${served.type}`,
    )

    await driver.findElement(By.xpath("//button[normalize-space(.)='Remove avatar']")).click()
    await driver.wait(
      async () => (await driver.findElements(By.css('img.avatar'))).length === 0,
      10000,
      'the removed avatar should stop rendering on the profile',
    )
    const after = await fetchStatus(driver, `${apiUrl}/api/users/${username}/avatar`)
    assert(after.status === 404, `removing should leave no avatar, got ${after.status}`)

    console.log('e2e: avatars flow passed')
  } finally {
    await driver.quit()
  }
}

run().catch((err) => {
  console.error(err)
  process.exit(1)
})
